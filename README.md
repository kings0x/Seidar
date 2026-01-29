📦 Project: Rust Production Reverse Proxy (v1)

A production-ready v1 reverse proxy built in Rust using Tokio and Axum.

This project prioritizes:

Correctness over cleverness

Operational safety over feature count

Clear subsystem boundaries

Predictable performance under load

This is not a toy proxy and not a full Envoy replacement.
It is a minimal, deployable, extensible reverse proxy suitable for real traffic.

🎯 Goals

Proxy HTTP/1.1, HTTP/2, and WebSocket traffic

Route requests to backend services

Provide load balancing, health checks, and timeouts

Be observable, debuggable, and resilient

Support safe configuration reloads

Shut down gracefully under load

🚫 Non-Goals (v1)

HTTP/3 / QUIC

Advanced L7 filtering

Full gRPC awareness

Complex policy engines

Distributed control plane

These belong in later versions.

🧠 Architectural Principles

Async-only (no blocking on runtime threads)

Fail fast, fail contained

Backpressure everywhere

Explicit state machines

No hidden global state

Config is data, not code

🏗️ High-Level Architecture

Client
→ Listener (TCP / TLS)
→ Protocol Layer (HTTP / WS)
→ Routing Engine
→ Load Balancer
→ Backend Pool
→ Response Pipeline
→ Client

Cross-cutting systems:

Observability

Health checking

Configuration management

Graceful shutdown

📁 Proposed Repository Layout
src/
  main.rs

  net/
    listener.rs
    connection.rs
    tls.rs

  http/
    server.rs
    request.rs
    response.rs
    websocket.rs

  routing/
    router.rs
    matcher.rs

  load_balancer/
    mod.rs
    round_robin.rs
    least_conn.rs
    backend.rs
    pool.rs

  health/
    active.rs
    passive.rs
    state.rs

  config/
    schema.rs
    loader.rs
    watcher.rs
    validation.rs

  observability/
    logging.rs
    metrics.rs
    tracing.rs

  resilience/
    timeouts.rs
    retries.rs
    circuit_breaker.rs

  security/
    headers.rs
    rate_limit.rs
    limits.rs

  lifecycle/
    startup.rs
    shutdown.rs
    signals.rs


No module exists without a reason.

🛣️ ENGINEERING ROADMAP (PHASED BUILD)

Each phase is independently testable.
Do not skip phases.

🔹 PHASE 0 — Specification & Skeleton

Goal: Lock architecture before writing logic.

Deliverables:

README (this document)

Config schema definition

Module skeletons

Explicit data flow diagrams (in comments or docs)

Constraints:

No business logic

No routing

No networking beyond placeholders

Why this matters:

Most proxy failures are architectural, not algorithmic.

🔹 PHASE 1 — Network & HTTP Foundation

Goal: Accept traffic safely.

Implement:

TCP listener

Connection lifecycle tracking

HTTP/1.1 + HTTP/2 via Axum

Request ID generation

Header size limits

Idle connection timeouts

Non-goals:

No routing

No backend calls yet

Success criteria:

Handles concurrent clients

No memory growth under idle load

🔹 PHASE 2 — Routing Engine

Goal: Decide where traffic goes.

Implement:

Host-based routing

Path-based routing

Deterministic rule matching

Route compilation at startup

Constraints:

No regex backtracking

No runtime allocations in hot path

Success criteria:

Predictable routing latency

Clear rejection on no-match

🔹 PHASE 3 — Backend Pool & Load Balancing

Goal: Forward traffic efficiently.

Implement:

Backend abstraction

Connection pooling

Round-robin balancing

Least-connections balancing

Per-backend limits

Constraints:

Bounded pools only

No connection leaks

Success criteria:

Stable under backend churn

No thundering herd

🔹 PHASE 4 — Health Checking & Failure Handling

Goal: Survive backend failure.

Implement:

Active health checks

Passive failure detection

Backend state transitions

Exclusion of unhealthy backends

Constraints:

No flapping

Gradual recovery

Success criteria:

Traffic avoids dead backends automatically

🔹 PHASE 5 — Timeouts, Retries & Resilience

Goal: Prevent cascading failures.

Implement:

Request timeouts

Backend response timeouts

Idempotent retry logic

Retry budgets

Constraints:

Never retry non-idempotent requests

Jittered backoff

Success criteria:

No retry storms

Fail fast under load

🔹 PHASE 6 — Observability

Goal: Make the system operable.

Implement:

Structured logs

Metrics (RPS, latency, errors)

Backend health metrics

Correlation IDs

Optional:

Distributed tracing hooks

Success criteria:

Every failure is explainable

🔹 PHASE 7 — Security & Abuse Protection

Goal: Assume hostile traffic.

Implement:

TLS termination

X-Forwarded-* headers

Rate limiting (per IP)

Max connections

Input validation

Constraints:

No trust in client input

No unbounded resource usage

Success criteria:

Proxy remains stable under abuse

🔹 PHASE 8 — Configuration & Hot Reload

Goal: Change behavior without downtime.

Implement:

Declarative config

Validation

Atomic reload

Rollback on failure

Constraints:

No traffic interruption

No partial config states

Success criteria:

Safe runtime reconfiguration

🔹 PHASE 9 — Graceful Shutdown & Lifecycle

Goal: Exit without chaos.

Implement:

Signal handling

Stop accepting new traffic

Drain in-flight requests

Forced shutdown deadline

Success criteria:

Zero dropped in-flight requests during normal shutdown

🔹 PHASE 10 — Hardening & Load Testing

Goal: Prove it survives reality.

Implement:

Load testing

Failure injection

Resource exhaustion tests

Latency profiling

Success criteria:

Stable p99 latency

No memory leaks

Predictable failure behavior

🧪 Definition of “Production-Ready v1”

This proxy is production-ready if:

It survives backend failures

It shuts down cleanly

It exposes enough telemetry to debug issues

It enforces limits consistently

It does not panic on malformed input