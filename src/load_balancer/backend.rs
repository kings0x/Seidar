//! Backend abstraction.
//!
//! # Responsibilities
//! - Represent a single backend server
//! - Track active connections (for Least Connections LB)
//! - Enforce max connection limits
//! - Track health state (Healthy/Unhealthy)

use url::Url;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, AtomicU8, Ordering};
use std::sync::Arc;
use std::ops::Deref;

/// Health State enum.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthState {
    Unknown = 0,
    Healthy = 1,
    Unhealthy = 2,
}

impl From<u8> for HealthState {
    fn from(val: u8) -> Self {
        match val {
            1 => HealthState::Healthy,
            2 => HealthState::Unhealthy,
            _ => HealthState::Unknown,
        }
    }
}

/// A single backend server.
#[derive(Debug)]
pub struct Backend {
    /// The address of the backend.
    pub addr: SocketAddr,
    /// Pre-calculated base URL for performance.
    pub base_url: Url,
    /// Maximum concurrent connections allowed.
    pub max_connections: usize,
    /// Number of currently active connections.
    pub active_connections: AtomicUsize,
    
    /// Current health state (0=Unknown, 1=Healthy, 2=Unhealthy).
    pub state: AtomicU8,
    /// Consecutive failure count.
    pub consecutive_failures: AtomicUsize,
    /// Consecutive success count.
    pub consecutive_successes: AtomicUsize,
}

impl Backend {
    /// Create a new backend.
    pub fn new(addr: SocketAddr, max_connections: usize) -> Self {
        let base_url = Url::parse(&format!("http://{}", addr)).unwrap();
        Self {
            addr,
            base_url,
            max_connections,
            active_connections: AtomicUsize::new(0),
            state: AtomicU8::new(HealthState::Unknown as u8),
            consecutive_failures: AtomicUsize::new(0),
            consecutive_successes: AtomicUsize::new(0),
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
    
    // --- Health Logic ---

    /// Return true if backend is considered healthy (Healthy or Unknown).
    pub fn is_healthy(&self) -> bool {
        let s = self.state.load(Ordering::Relaxed);
        s != (HealthState::Unhealthy as u8)
    }

    /// Report a successful request/check.
    pub fn mark_success(&self, healthy_threshold: usize) {
        // Reset failures
        self.consecutive_failures.store(0, Ordering::Relaxed);
        
        let current_state = self.state.load(Ordering::Relaxed);
        if current_state == (HealthState::Healthy as u8) {
            // Already healthy, just ensure failures 0
            return;
        }

        // Increment successes
        let successes = self.consecutive_successes.fetch_add(1, Ordering::Relaxed) + 1;
        
        if successes >= healthy_threshold {
            // Transition to Healthy
            self.state.store(HealthState::Healthy as u8, Ordering::Relaxed);
            // Optional: log transition? (Need logger access, maybe return bool indicating transition)
        }
    }

    /// Report a failed request/check.
    pub fn mark_failure(&self, unhealthy_threshold: usize) {
        // Reset successes
        self.consecutive_successes.store(0, Ordering::Relaxed);

        let current_state = self.state.load(Ordering::Relaxed);
        if current_state == (HealthState::Unhealthy as u8) {
            return;
        }

        // Increment failures
        let failures = self.consecutive_failures.fetch_add(1, Ordering::Relaxed) + 1;

        if failures >= unhealthy_threshold {
            // Transition to Unhealthy
            self.state.store(HealthState::Unhealthy as u8, Ordering::Relaxed);
        }
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
