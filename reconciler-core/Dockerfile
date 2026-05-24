# =============================================================
# Reconciler API – Multi-stage Rust build
# =============================================================

# ─────────────────────────────────────────────────
# Stage 1: Dependency cache layer
# Pre-build dependencies to leverage Docker layer cache
# ─────────────────────────────────────────────────
FROM rust:latest AS deps

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    libpq-dev \
    binutils \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first for dependency caching
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to compile dependencies only
RUN mkdir -p src && echo "fn main() {}" > src/main.rs

# Build dependencies only (this layer is cached unless Cargo.toml/lock changes)
RUN cargo build --release 2>&1 && \
    rm -f target/release/deps/reconciler*

# ─────────────────────────────────────────────────
# Stage 2: Builder
# Compile the actual application
# ─────────────────────────────────────────────────
FROM deps AS builder

WORKDIR /app

# Copy the full source tree
COPY src ./src
COPY migrations ./migrations

# Touch main.rs to invalidate the dummy build
RUN touch src/main.rs

# Build release binary
RUN cargo build --release 2>&1

# Strip debug symbols to reduce binary size (~50% smaller)
RUN strip target/release/reconciler

# ─────────────────────────────────────────────────
# Stage 3: Runtime
# Minimal image with just the binary
# ─────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies only
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    libpq5 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN groupadd --gid 1001 reconciler && \
    useradd --uid 1001 --gid 1001 --no-create-home --shell /bin/false reconciler

WORKDIR /app

# Copy binary from builder stage
COPY --from=builder /app/target/release/reconciler /app/reconciler

# Copy migrations (needed at runtime for auto-migrate)
COPY --from=builder /app/migrations /app/migrations

# Set ownership
RUN chown -R reconciler:reconciler /app

# Switch to non-root user
USER reconciler

# Expose API port
EXPOSE 8080

# Health check – hit the /health endpoint
HEALTHCHECK --interval=10s --timeout=5s --start-period=15s --retries=3 \
    CMD curl -sf http://localhost:8080/health || exit 1

# Run the binary
ENTRYPOINT ["/app/reconciler"]
