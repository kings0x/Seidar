# Developer Integration Guide

This guide explains how to integrate your application with the Blockchain-Enabled Reverse Proxy.

## Authentication Overview

The proxy uses **Blockchain Addresses** as the primary identity. 
1. Users must provide their address via the `X-User-Address` header.
2. The proxy verifies if that address has an active subscription in its local cache (which is updated via the `PaymentMonitor`).

## Step-by-Step Integration

### 1. Check Subscription Status
Before making requests, ensure the user has a valid subscription. You can check this by making a trial request or checking on-chain state directly.

### 2. Request a Quote
If the user needs a subscription or renewal, request a quote through the proxy:

**Endpoint**: `POST /api/v1/quote`

```json
{
  "service_type": "tier_1",
  "user_address": "0x123...",
  "duration_seconds": 2592000
}
```

The proxy returns a **Signed Quote** which must be passed to the `PaymentProcessor` smart contract on-chain.

### 3. Make a Payment
Submit the signed quote to the `PaymentProcessor::buySubscription` function on the LitVM testnet.

### 4. Perform Proxied Requests
Once the payment is confirmed on-chain (usually within 3 blocks), the `PaymentMonitor` will update the proxy's local cache. You can now perform requests:

```bash
curl -H "X-User-Address: 0x123..." http://proxy-url/your-api-path
```

## Using the SDKs

### Rust
```rust
let client = ProxyClient::new("http://localhost:8080");
let res = client.proxy_get("/data", "0x123...").await?;
```

### TypeScript
```typescript
const client = new ProxyClient("http://localhost:8080");
const res = await client.proxyGet("/data", "0x123...");
```

## Quality of Service (QoS) Tiers

The proxy enforces rate limits and connection limits based on tiers:

| Tier | RPS | Max Connections (WS/SSE) |
| :--- | :--- | :--- |
| **Free/None** | 1 | N/A |
| **Tier 1** | 10 | 1 |
| **Tier 2** | 100 | 10 |
| **Tier 3** | 1000 | 1000 |

## Headers

| Header | Description | Required |
| :--- | :--- | :--- |
| `X-User-Address` | The user's blockchain address (0x...). | Yes (for auth) |
| `X-Request-ID` | Unique ID for tracing the request. | Optional |
