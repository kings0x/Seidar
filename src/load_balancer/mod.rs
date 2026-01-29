//! Load balancing subsystem.
//!
//! # Data Flow
//! ```text
//! Route matched → backend_group identified
//!     → pool.rs (get available backends)
//!     → Apply load balancing algorithm:
//!         - round_robin.rs (rotate through backends)
//!         - least_conn.rs (pick backend with fewest connections)
//!     → backend.rs (acquire connection from pool)
//!     → Return backend connection or error
//! ```
//!
//! # Design Decisions
//! - Load balancer is stateless; backend pool tracks connections
//! - Algorithm selection per backend group
//! - Unhealthy backends excluded from selection
//! - Connection pooling per backend for efficiency

pub mod backend;
pub mod least_conn;
pub mod pool;
pub mod round_robin;
