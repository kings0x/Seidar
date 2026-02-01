# LitVM Reverse Proxy - Smart Contracts

This directory contains Foundry-based smart contracts for the blockchain-enabled reverse proxy.

## Prerequisites

- [Foundry](https://getfoundry.sh/) installed
- Access to LitVM RPC endpoint

## Quick Start

```bash
# Install dependencies
forge install

# Build contracts
forge build

# Run tests
forge test

# Run tests with gas reporting
forge test --gas-report
```

## Contract Overview

| Contract | Purpose |
|----------|---------|
| `SubscriptionManager.sol` | Core subscription lifecycle management |
| `PaymentProcessor.sol` | Payment handling and event emission |
| `AccessToken.sol` | On-chain access validation |

## Environment Setup

Copy `.env.example` to `.env` and fill in:

```bash
cp .env.example .env
```

Required variables:
- `PRIVATE_KEY` - Deployer private key
- `LITVM_RPC_URL` - LitVM mainnet RPC endpoint
- `LITVM_TESTNET_RPC_URL` - LitVM testnet RPC endpoint

## Deployment

```bash
# Deploy to local Anvil
forge script script/Deploy.s.sol --rpc-url anvil --broadcast

# Deploy to LitVM testnet
forge script script/Deploy.s.sol --rpc-url litvm_testnet --broadcast --verify
```

## Security

⚠️ **Never commit `.env` files with private keys**
