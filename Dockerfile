# Multi-stage build for Scryfall Cache Microservice

# Stage 1: Builder
FROM rust:slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy all source files
COPY Cargo.toml ./
COPY Cargo.lock ./
COPY src ./src
COPY migrations ./migrations

# Build the application in release mode
# Set SQLX_OFFLINE to skip compile-time SQL verification
ENV SQLX_OFFLINE=true
ARG CARGO_FEATURES=""
RUN if [ -n "$CARGO_FEATURES" ]; then \
      cargo build --release --features "$CARGO_FEATURES"; \
    else \
      cargo build --release; \
    fi

# Stage 2: Runtime
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    wget \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN groupadd -g 1000 appuser && \
    useradd -u 1000 -g appuser -s /bin/bash -m appuser

# Create app directory
WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/scryfall-cache /app/scryfall-cache

# Copy migrations
COPY --from=builder /app/migrations /app/migrations

# Change ownership
RUN chown -R appuser:appuser /app

# Switch to app user
USER appuser

# Expose the API port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=40s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:8080/health || exit 1

# Run the binary
CMD ["/app/scryfall-cache"]
