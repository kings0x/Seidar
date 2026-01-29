//! Configuration validation logic.

use crate::config::schema::ProxyConfig;
use std::collections::HashSet;

/// Error type for configuration validation failures.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError(pub String);

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Validation error: {}", self.0)
    }
}

/// Validate a ProxyConfig for semantic correctness.
pub fn validate_config(config: &ProxyConfig) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    // 1. Check referential integrity: Routes must point to existing backend groups
    let backend_groups: HashSet<&str> = config.backends.iter().map(|b| b.group.as_str()).collect();
    
    for route in &config.routes {
        if !backend_groups.contains(route.backend_group.as_str()) {
            errors.push(ValidationError(format!(
                "Route '{}' references unknown backend group '{}'",
                route.name, route.backend_group
            )));
        }
    }

    // 2. Validate thresholds
    if config.health_check.healthy_threshold == 0 {
        errors.push(ValidationError("health_check.healthy_threshold must be > 0".to_string()));
    }
    if config.health_check.unhealthy_threshold == 0 {
        errors.push(ValidationError("health_check.unhealthy_threshold must be > 0".to_string()));
    }

    // 3. Validate retry budget
    if config.retries.budget_ratio < 0.0 || config.retries.budget_ratio > 1.0 {
        errors.push(ValidationError("retries.budget_ratio must be between 0.0 and 1.0".to_string()));
    }

    // 4. Validate timeouts (basic check)
    if config.timeouts.connect_secs == 0 && config.timeouts.request_secs == 0 {
        // Technically they could be 0 but likely a mistake
        tracing::warn!("Timeouts are set to 0, matching requests might time out immediately");
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::*;

    #[test]
    fn test_valid_config() {
        let mut config = ProxyConfig::default();
        config.backends.push(BackendConfig {
            name: "b1".into(),
            group: "web".into(),
            address: "127.0.0.1:80".into(),
            weight: 1,
            max_connections: 100,
        });
        config.routes.push(RouteConfig {
            name: "r1".into(),
            host: None,
            path_prefix: Some("/".into()),
            backend_group: "web".into(),
            priority: 0,
        });

        assert!(validate_config(&config).is_ok());
    }

    #[test]
    fn test_invalid_backend_group() {
        let mut config = ProxyConfig::default();
        config.routes.push(RouteConfig {
            name: "r1".into(),
            host: None,
            path_prefix: Some("/".into()),
            backend_group: "missing".into(),
            priority: 0,
        });

        let errs = validate_config(&config).unwrap_err();
        assert_eq!(errs.len(), 1);
        assert!(errs[0].0.contains("unknown backend group 'missing'"));
    }
}
