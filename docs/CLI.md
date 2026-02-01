# CLI Management Tool Guide

The `proxy-cli` is a terminal-based utility to monitor and manage the Rust Reverse Proxy.

## Installation

Ensure you have the Rust toolchain installed, then build the CLI:

```bash
cargo build --bin proxy-cli --release
```

The binary will be located at `target/release/proxy-cli`.

## Configuration

By default, the CLI expects the proxy's admin API at `http://localhost:8081` with the key `admin-secret-key`. You can override these via flags:

```bash
proxy-cli --url "http://127.0.0.1:8081" --key "your-custom-key" <command>
```

## Commands

### 1. Check System Status
Verify if the proxy is operational and see the current version.
```bash
proxy-cli status
```

### 2. Inspect Backends
Get a real-time list of all backend servers, their health status (healthy/unhealthy), and active connection counts.
```bash
proxy-cli backends
```

### 3. View Analytics
See aggregated metrics such as total request count and active long-lived tunnels.
```bash
proxy-cli analytics
```

### 4. Inspect Cache
Get a summary of the current subscription cache (active blockchain sessions).
```bash
proxy-cli cache
```

## Troubleshooting

- **Connection Refused**: Ensure the proxy is running and the `admin.enabled` setting in `config.toml` is `true`.
- **401 Unauthorized**: The --key flag must match the `admin.api_key` in the proxy's configuration.
- **Connection Timeout**: Verify there are no firewalls blocking the admin port (default 8081).
