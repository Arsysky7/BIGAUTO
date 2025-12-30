# Multi-stage build untuk optimize image size
FROM rust:1.90-slim as builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY shared ./shared
COPY services/payment-service ./services/payment-service

# Build service
RUN cargo build --release -p payment-service

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy binary dari builder
COPY --from=builder /app/target/release/payment-service /usr/local/bin/payment-service

EXPOSE 3005

ENV RUST_LOG=info
ENV RUST_ENV=production

CMD ["payment-service"]