//! Blockchain integration subsystem.
//!
//! # Data Flow
//! ```text
//! Environment Variables (private key, RPC URL)
//!     → wallet.rs (key loading, signing)
//!     → client.rs (RPC connection with timeouts)
//!     → transaction.rs (build, sign, broadcast, confirm)
//! ```
//!
//! # Security Constraints
//! - Private keys ONLY from environment variables
//! - Never log private keys or sensitive data
//! - All RPC calls have configurable timeouts
//! - Graceful degradation when blockchain unreachable

pub mod client;
pub mod transaction;
pub mod types;
pub mod wallet;

pub use client::BlockchainClient;
pub use types::{BlockchainConfig, BlockchainError, ChainId};
pub use wallet::Wallet;
