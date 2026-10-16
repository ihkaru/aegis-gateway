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
RUN cargo build --release --bin propylon

# Stage 3: Minimal, Secure Runtime Image
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies for sandbox execution and mise toolchain management
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    git \
    python3 \
    tar \
    unzip \
    xz-utils \
    zstd \
    && (for i in 1 2 3 4 5; do curl -fsSL https://mise.run | MISE_INSTALL_PATH=/usr/local/bin/mise sh && break || sleep 2; done)

# Create non-root unprivileged user
RUN groupadd -g 10001 propylon && \
    useradd -u 10001 -g propylon -s /bin/bash -m -d /home/propylon propylon

WORKDIR /app

# Copy compiled binary from builder stage
COPY --from=builder /app/target/release/propylon /usr/local/bin/propylon

# Create runtime directories with appropriate permissions
RUN mkdir -p /app/data/mise /app/data/bin /tmp/propylon-cache /home/propylon && \
    chown -R propylon:propylon /app /tmp/propylon-cache /home/propylon

USER propylon

ENV PROPYLON_HOST=0.0.0.0 \
    PROPYLON_PORT=8080 \
    PORT=8080 \
    HOME=/home/propylon \
    RUST_LOG=propylon=info,warn \
    MISE_DATA_DIR=/app/data/mise \
    MISE_CACHE_DIR=/app/data/mise/cache \
    MISE_CONFIG_DIR=/app/data/mise/config \
    MISE_STATE_DIR=/app/data/mise/state \
    MISE_YES=1 \
    PATH="/app/data/mise/shims:/app/data/mise/bin:/app/data/bin:/home/propylon/.local/bin:${PATH}"

EXPOSE 8080

HEALTHCHECK --interval=20s --timeout=5s --retries=3 --start-period=10s \
    CMD curl -f http://localhost:8080/healthz || exit 1

ENTRYPOINT ["/usr/local/bin/propylon"]
CMD ["serve"]
