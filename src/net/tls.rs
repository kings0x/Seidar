//! TLS configuration and certificate loading.

use std::path::Path;
use axum_server::tls_rustls::RustlsConfig;

/// Load TLS configuration from certificate and key files.
pub async fn load_tls_config(cert_path: &Path, key_path: &Path) -> Result<RustlsConfig, std::io::Error> {
    // Basic validation
    if !cert_path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Certificate file not found: {:?}", cert_path),
        ));
    }
    if !key_path.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Private key file not found: {:?}", key_path),
        ));
    }

    // Load cert and key using axum-server's helper if possible, 
    // or manually if we need more control.
    // axum-server::tls_rustls::RustlsConfig::from_pem_file is convenient.
    RustlsConfig::from_pem_file(cert_path, key_path).await
}
