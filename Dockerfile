# Semantica Task Engine - Production Dockerfile
# Phase 4: Container deployment

# Build stage
FROM rust:1.82-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY rustfmt.toml ./
COPY crates ./crates

# Build release with telemetry
RUN cargo build --release --features telemetry

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    sqlite3 \
    && rm -rf /var/lib/apt/lists/*

# Create user
RUN useradd -m -u 1000 semantica

# Create data directory
RUN mkdir -p /var/lib/semantica && \
    chown semantica:semantica /var/lib/semantica

# Copy binaries
COPY --from=builder /build/target/release/semantica /usr/local/bin/
COPY --from=builder /build/target/release/semantica-cli /usr/local/bin/

# Switch to non-root user
USER semantica
WORKDIR /home/semantica

# Environment
ENV SEMANTICA_DB_PATH=/var/lib/semantica/meta.db
ENV SEMANTICA_RPC_PORT=9527
ENV SEMANTICA_LOG_FORMAT=json
ENV RUST_LOG=semantica=info

# Expose RPC port
EXPOSE 9527

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD [ "sh", "-c", "curl -sf http://localhost:${SEMANTICA_RPC_PORT}/ || exit 1" ]

# Volume for persistent data
VOLUME ["/var/lib/semantica"]

# Run daemon
CMD ["semantica"]

