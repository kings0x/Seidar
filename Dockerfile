# --- Build Stage ---
FROM rust:1.80-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

# Copy the entire workspace
COPY . .

# Build the application in release mode
RUN cargo build --release

# --- Runtime Stage ---
FROM debian:bookworm-slim

# Install runtime dependencies (OpenSSL is needed for HTTPS)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create a non-privileged user to run the app
RUN groupadd -r proxyapp && useradd -r -g proxyapp proxyapp

WORKDIR /app

# Copy the binary from the builder stage
COPY --from=builder /usr/src/app/target/release/reverse-proxy /app/reverse-proxy

# Copy default config if it exists, otherwise users should mount it
COPY --from=builder /usr/src/app/config.toml /app/config.toml

# Set ownership to the non-privileged user
RUN chown -R proxyapp:proxyapp /app

USER proxyapp

# Expose ports based on default config (8080 for proxy, 8081 for admin)
EXPOSE 8080
EXPOSE 8081

# Command to run the application
ENTRYPOINT ["/app/reverse-proxy"]
CMD ["--config", "/app/config.toml"]
