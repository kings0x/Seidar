//! Configuration loading from disk.

use std::path::Path;
use std::fs;
use crate::config::schema::ProxyConfig;
use crate::config::validation::{validate_config, ValidationError};

/// Error type for configuration loading.
#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Parse(toml::de::Error),
    Validation(Vec<ValidationError>),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "IO error: {}", e),
            ConfigError::Parse(e) => write!(f, "Parse error: {}", e),
            ConfigError::Validation(errors) => {
                write!(f, "Validation failed: ")?;
                for (i, err) in errors.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", err)?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ConfigError {}

/// Load and validate configuration from a TOML file.
pub fn load_config(path: &Path) -> Result<ProxyConfig, ConfigError> {
    let content = fs::read_to_string(path).map_err(ConfigError::Io)?;
    let config: ProxyConfig = toml::from_str(&content).map_err(ConfigError::Parse)?;
    
    validate_config(&config).map_err(ConfigError::Validation)?;
    
    Ok(config)
}
