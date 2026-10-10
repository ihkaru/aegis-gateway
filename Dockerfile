# ==============================================================================
# Multi-Stage Dockerfile with Cargo-Chef Layer Caching
# Designed for Fast GitHub App Auto-Deployments in Coolify & Enterprise Pods
# ==============================================================================

# Stage 1: Cargo Chef Planner
FROM lukemathwalker/cargo-chef:latest-rust-1.93-bookworm AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 2: Cache & Build Dependencies Only
FROM chef AS builder
WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this layer is cached by Docker across deployments
RUN cargo chef cook --release --recipe-path recipe.json

# Copy application source code and compile binary
COPY . .
RUN cargo build --release --bin aegis-gateway

# Stage 3: Minimal, Secure Runtime Image
FROM debian:bookworm-slim AS runtime

# Install CA certificates, curl, and python3 for code sandbox execution
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    python3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root unprivileged user
RUN groupadd -g 10001 aegis && \
    useradd -u 10001 -g aegis -s /bin/false -m aegis

WORKDIR /app

# Copy compiled binary from builder stage
COPY --from=builder /app/target/release/aegis-gateway /usr/local/bin/aegis-gateway

# Create runtime directories with appropriate permissions
RUN mkdir -p /app/data /tmp/aegis-cache && \
    chown -R aegis:aegis /app /tmp/aegis-cache

USER aegis

ENV AEGIS_HOST=0.0.0.0 \
    AEGIS_PORT=8080 \
    PORT=8080 \
    RUST_LOG=aegis_gateway=info,warn

EXPOSE 8080

HEALTHCHECK --interval=20s --timeout=5s --retries=3 --start-period=10s \
    CMD curl -f http://localhost:8080/healthz || exit 1

ENTRYPOINT ["/usr/local/bin/aegis-gateway"]
CMD ["serve"]
