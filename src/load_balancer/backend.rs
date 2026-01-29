//! Backend abstraction.
//!
//! # Responsibilities
//! - Represent a single backend server
//! - Track active connections (for Least Connections LB)
//! - Enforce max connection limits

use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::ops::Deref;

/// A single backend server.
#[derive(Debug)]
pub struct Backend {
    /// The address of the backend.
    pub addr: SocketAddr,
    /// Maximum concurrent connections allowed.
    pub max_connections: usize,
    /// Number of currently active connections.
    pub active_connections: AtomicUsize,
}

impl Backend {
    /// Create a new backend.
    pub fn new(addr: SocketAddr, max_connections: usize) -> Self {
        Self {
            addr,
            max_connections,
            active_connections: AtomicUsize::new(0),
        }
    }

    /// Get the current number of active connections.
    pub fn loop_count(&self) -> usize {
        self.active_connections.load(Ordering::Relaxed)
    }

    /// Increment active connection count.
    pub fn inc_connections(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement active connection count.
    pub fn dec_connections(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    /// Try to create a connection guard that increments count.
    /// Returns None if max_connections reached.
    pub fn try_create_guard(self: &Arc<Self>) -> Option<BackendConnectionGuard> {
        let mut prev = self.active_connections.load(Ordering::Relaxed);
        loop {
            if prev >= self.max_connections {
                return None;
            }
            match self.active_connections.compare_exchange_weak(
                prev, prev + 1, Ordering::Relaxed, Ordering::Relaxed
            ) {
                Ok(_) => break,
                Err(x) => prev = x,
            }
        }
        Some(BackendConnectionGuard {
            backend: self.clone(),
        })
    }
}

/// A RAII guard that manages the active connection count.
#[derive(Debug)]
pub struct BackendConnectionGuard {
    pub backend: Arc<Backend>,
}

impl Deref for BackendConnectionGuard {
    type Target = Backend;
    fn deref(&self) -> &Self::Target {
        &self.backend
    }
}

impl Drop for BackendConnectionGuard {
    fn drop(&mut self) {
        self.backend.dec_connections();
    }
}
