//! TCP listener implementation with backpressure.
//!
//! # Responsibilities
//! - Bind to configured address(es)
//! - Accept incoming TCP connections
//! - Enforce max_connections limit via semaphore
//! - Graceful handling of accept errors

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;

use crate::config::ListenerConfig;

/// Error type for listener operations.
#[derive(Debug)]
pub enum ListenerError {
    /// Failed to bind to address.
    Bind(std::io::Error),
    /// Failed to accept connection.
    Accept(std::io::Error),
}

impl std::fmt::Display for ListenerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ListenerError::Bind(e) => write!(f, "Failed to bind: {}", e),
            ListenerError::Accept(e) => write!(f, "Failed to accept: {}", e),
        }
    }
}

impl std::error::Error for ListenerError {}

/// A bounded TCP listener that limits concurrent connections.
///
/// Uses a semaphore to enforce `max_connections`. When the limit is reached,
/// new connections will wait until a slot becomes available.
pub struct Listener {
    /// The underlying TCP listener.
    inner: TcpListener,
    /// Semaphore to limit concurrent connections.
    connection_limit: Arc<Semaphore>,
    /// Configured maximum connections.
    max_connections: usize,
}

impl Listener {
    /// Bind to the configured address with connection limits.
    pub async fn bind(config: &ListenerConfig) -> Result<Self, ListenerError> {
        let addr: SocketAddr = config
            .bind_address
            .parse()
            .map_err(|e| ListenerError::Bind(std::io::Error::new(std::io::ErrorKind::InvalidInput, e)))?;

        let listener = TcpListener::bind(addr)
            .await
            .map_err(ListenerError::Bind)?;

        let local_addr = listener
            .local_addr()
            .map_err(ListenerError::Bind)?;

        tracing::info!(
            address = %local_addr,
            max_connections = config.max_connections,
            "Listener bound"
        );

        Ok(Self {
            inner: listener,
            connection_limit: Arc::new(Semaphore::new(config.max_connections)),
            max_connections: config.max_connections,
        })
    }

    /// Accept a new connection, respecting the connection limit.
    ///
    /// This will wait if the connection limit has been reached.
    /// Returns the stream and a permit that must be held for the connection's lifetime.
    pub async fn accept(&self) -> Result<(TcpStream, SocketAddr, ConnectionPermit), ListenerError> {
        // Acquire permit first (backpressure)
        let permit = self
            .connection_limit
            .clone()
            .acquire_owned()
            .await
            .expect("Semaphore closed unexpectedly");

        // Then accept the connection
        let (stream, addr) = self.inner.accept().await.map_err(ListenerError::Accept)?;

        tracing::debug!(
            peer_addr = %addr,
            available_permits = self.connection_limit.available_permits(),
            "Connection accepted"
        );

        Ok((stream, addr, ConnectionPermit { _permit: permit }))
    }

    /// Get the local address this listener is bound to.
    pub fn local_addr(&self) -> Result<SocketAddr, std::io::Error> {
        self.inner.local_addr()
    }

    /// Get current available connection slots.
    pub fn available_permits(&self) -> usize {
        self.connection_limit.available_permits()
    }

    /// Get configured maximum connections.
    pub fn max_connections(&self) -> usize {
        self.max_connections
    }
}

/// A permit representing a connection slot.
///
/// When dropped, the connection slot is released back to the pool.
/// This ensures backpressure is maintained even if the connection handler panics.
#[derive(Debug)]
pub struct ConnectionPermit {
    _permit: tokio::sync::OwnedSemaphorePermit,
}

impl ConnectionPermit {
    /// Check if the permit is still valid (always true while held).
    pub fn is_valid(&self) -> bool {
        true
    }
}
