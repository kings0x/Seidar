# Monitoring & Alerting Guide

This project includes a built-in monitoring stack based on **Prometheus** and **Grafana**.

## Architecture

1. **Proxy**: Exposes Prometheus-compatible metrics on port `9090` (by default).
2. **Prometheus**: Scrapes the proxy metrics and stores them as time-series data.
3. **Grafana**: Visualizes the data stored in Prometheus.

## Getting Started

1. **Start the Stack**:
   ```bash
   docker-compose up -d
   ```

2. **Access Prometheus**:
   Open `http://localhost:9090`. You can query metrics like `proxy_requests_total` directly.

3. **Access Grafana**:
   Open `http://localhost:3000`. 
   - Anonymous access is enabled as Admin (for development).
   - Dashboards are automatically provisioned (see below).

## Dashboards

A pre-configured dashboard is available at `http://localhost:3000/dashboards`.

### Key Metrics Tracked
- **Request Rate**: HTTP requests per second split by method and status code.
- **WebSocket/SSE Connections**: Real-time count of active long-lived tunnels.
- **Rate Limiting**: Count of requests blocked due to RPS or connection limits.
- **Backend Health**: Binary status (1 for healthy, 0 for unhealthy) for each backend group.
- **Cache Size**: Number of unique user subscriptions tracked in memory.

## Alerting

While this v1 project provides the infrastructure for alerting (via Prometheus evaluation), users should configure their own alert rules in `prometheus.yml` or using Grafana Alerts.

### Recommended Alerts
- **High Error Rate**: `rate(proxy_requests_total{status=~"5.."}[5m]) > 0.05`
- **Backend Down**: `proxy_backend_healthy == 0`
- **Connection Limit Reached**: `rate(proxy_rate_limited_total{reason="websocket_limit"}[5m]) > 0`
