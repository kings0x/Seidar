//! Shutdown coordination for the proxy.

use tokio::sync::broadcast;

/// Coordinator for graceful shutdown.
///
/// Provides a broadcast channel that all long-running tasks can subscribe to.
pub struct Shutdown {
    /// Broadcast channel sender.
    tx: broadcast::Sender<()>,
}

impl Shutdown {
    /// Create a new shutdown coordinator.
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1);
        Self { tx }
    }

    /// Subscribe to the shutdown signal.
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.tx.subscribe()
    }

    /// Trigger the shutdown signal.
    pub fn trigger(&self) {
        let _ = self.tx.send(());
    }

    /// Get the number of active subscribers (tasks still running).
    pub fn receiver_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

impl Default for Shutdown {
    fn default() -> Self {
        Self::new()
    }
}
