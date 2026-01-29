//! Configuration file watcher for hot reload.

use std::path::{Path, PathBuf};
use std::time::Duration;
use notify::{Watcher, RecursiveMode, Event, RecommendedWatcher, Config};
use tokio::sync::mpsc;
use crate::config::loader::load_config;
use crate::config::schema::ProxyConfig;

/// A watcher that monitors the configuration file for changes.
pub struct ConfigWatcher {
    path: PathBuf,
    update_tx: mpsc::UnboundedSender<ProxyConfig>,
}

impl ConfigWatcher {
    /// Create a new ConfigWatcher.
    /// 
    /// Returns the watcher and a receiver for configuration updates.
    pub fn new(path: &Path) -> (Self, mpsc::UnboundedReceiver<ProxyConfig>) {
        let (update_tx, update_rx) = mpsc::unbounded_channel();
        
        (Self {
            path: path.to_path_buf(),
            update_tx,
        }, update_rx)
    }

    /// Start watching the file in a background thread.
    pub fn run(self) -> Result<RecommendedWatcher, notify::Error> {
        let tx = self.update_tx.clone();
        let path = self.path.clone();

        let mut watcher = RecommendedWatcher::new(move |res: notify::Result<Event>| {
            match res {
                Ok(event) => {
                    if event.kind.is_modify() || event.kind.is_create() {
                        tracing::info!("Config file change detected, reloading...");
                        match load_config(&path) {
                            Ok(new_config) => {
                                let _ = tx.send(new_config);
                            }
                            Err(e) => {
                                tracing::error!("Failed to reload config: {}. Keeping current configuration.", e);
                            }
                        }
                    }
                }
                Err(e) => tracing::error!("Watch error: {:?}", e),
            }
        }, Config::default().with_poll_interval(Duration::from_secs(2)))?;

        watcher.watch(&self.path, RecursiveMode::NonRecursive)?;
        
        tracing::info!(path = ?self.path, "Config watcher started");
        Ok(watcher)
    }
}
