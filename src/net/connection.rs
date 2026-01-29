//! Connection state machine and lifecycle tracking.
//!
//! # Responsibilities
//! - Track connection state (Active → Draining → Closed)
//! - Generate unique connection IDs for tracing
//! - Coordinate graceful shutdown per-connection
//! - Collect per-connection metrics

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::watch;

/// Global atomic counter for connection IDs.
/// Using relaxed ordering is sufficient since we only need uniqueness, not synchronization.
static CONNECTION_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Unique identifier for a connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionId(u64);

impl ConnectionId {
    /// Generate a new unique connection ID.
    pub fn new() -> Self {
        Self(CONNECTION_ID_COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Get the raw ID value.
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl Default for ConnectionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "conn-{}", self.0)
    }
}

/// Connection state for lifecycle tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Connection is active and processing requests.
    Active,
    /// Connection is draining (no new requests, finishing in-flight).
    Draining,
    /// Connection is closed.
    Closed,
}

/// Tracks active connections for graceful shutdown.
///
/// Uses a watch channel to signal when all connections have closed.
#[derive(Debug, Clone)]
pub struct ConnectionTracker {
    /// Current count of active connections.
    active_count: Arc<AtomicU64>,
    /// Sender to signal when connections change.
    _shutdown_tx: Arc<watch::Sender<bool>>,
    /// Receiver to wait for shutdown completion.
    shutdown_rx: watch::Receiver<bool>,
}

impl ConnectionTracker {
    /// Create a new connection tracker.
    pub fn new() -> Self {
        let (tx, rx) = watch::channel(false);
        Self {
            active_count: Arc::new(AtomicU64::new(0)),
            _shutdown_tx: Arc::new(tx),
            shutdown_rx: rx,
        }
    }

    /// Record a new active connection. Returns a guard that decrements on drop.
    pub fn track(&self) -> ConnectionGuard {
        self.active_count.fetch_add(1, Ordering::SeqCst);
        ConnectionGuard {
            active_count: Arc::clone(&self.active_count),
            id: ConnectionId::new(),
        }
    }

    /// Get current active connection count.
    pub fn active_count(&self) -> u64 {
        self.active_count.load(Ordering::SeqCst)
    }

    /// Wait until all connections are closed or timeout.
    pub async fn wait_for_shutdown(&mut self) {
        // Wait until active count reaches 0
        while self.active_count.load(Ordering::SeqCst) > 0 {
            // Check periodically
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    /// Clone the shutdown receiver for use in tasks.
    pub fn shutdown_receiver(&self) -> watch::Receiver<bool> {
        self.shutdown_rx.clone()
    }
}

impl Default for ConnectionTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Guard that tracks a connection's lifetime.
/// Decrements active count when dropped.
#[derive(Debug)]
pub struct ConnectionGuard {
    active_count: Arc<AtomicU64>,
    id: ConnectionId,
}

impl ConnectionGuard {
    /// Get this connection's ID.
    pub fn id(&self) -> ConnectionId {
        self.id
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.active_count.fetch_sub(1, Ordering::SeqCst);
        tracing::trace!(connection_id = %self.id, "Connection closed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_id_unique() {
        let id1 = ConnectionId::new();
        let id2 = ConnectionId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn connection_tracker_counts() {
        let tracker = ConnectionTracker::new();
        assert_eq!(tracker.active_count(), 0);

        let guard1 = tracker.track();
        assert_eq!(tracker.active_count(), 1);

        let guard2 = tracker.track();
        assert_eq!(tracker.active_count(), 2);

        drop(guard1);
        assert_eq!(tracker.active_count(), 1);

        drop(guard2);
        assert_eq!(tracker.active_count(), 0);
    }
}
