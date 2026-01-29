//! Configuration file watcher for hot reload.
//!
//! # Responsibilities
//! - Watch config file for changes (inotify/kqueue)
//! - Debounce rapid changes to avoid reload storms
//! - Trigger reload pipeline on valid change detection
//!
//! # Design Decisions
//! - Uses async file watching to avoid blocking runtime threads
//! - Debounce window prevents partial-write reloads
//! - Errors in watching should log but not crash the proxy
