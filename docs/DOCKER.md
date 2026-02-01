# Docker Deployment Guide

This guide explains how to package and run the Rust Reverse Proxy using Docker.

## Prerequisites
- Docker Desktop (Running)
- Docker Compose v2+

## Quick Start (Compose)

1. **Configure Environment**:
   Copy the example environment file and fill in your details:
   ```bash
   cp .env.example .env
   ```

2. **Run with Compose**:
   ```bash
   docker-compose up -d --build
   ```

3. **Verify**:
   - Proxy: `curl http://localhost:8080`
   - Admin: `curl -H "Authorization: Bearer admin-secret-key" http://localhost:8081/admin/status`

## Manual Build & Run

If you want to build the image without compose:

### 1. Build
```bash
docker build -t reverse-proxy:latest .
```

### 2. Run
```bash
docker run -p 8080:8080 -p 8081:8081 \
  -v $(pwd)/config.toml:/app/config.toml \
  -e RUST_LOG=info \
  reverse-proxy:latest
```

## Production Considerations

### 1. Configuration Mounting
Always mount your `config.toml` from the host to `/app/config.toml` in the container to ensure configuration persists across restarts.

### 2. Subscription Persistence
Mount `subscriptions.json` to ensure the proxy doesn't lose subscription data when the container is recreated:
```yaml
volumes:
  - ./subscriptions.json:/app/subscriptions.json
```

### 3. Resource Limits
In production, it is recommended to set resource limits in your `docker-compose.yml`:
```yaml
deploy:
  resources:
    limits:
      cpus: '0.50'
      memory: 512M
```

### 4. Logging
Logs are sent to `stdout`. Use a logging driver (like `json-file` or `fluentd`) to manage log rotation and aggregation.
