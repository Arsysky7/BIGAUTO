FROM rust:1.90-slim as builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace files
COPY Cargo.toml ./
COPY shared ./shared

# Copy service files
COPY services/auth-service/Cargo.toml ./services/auth-service/
RUN mkdir -p services/auth-service/src && echo "fn main() {}" > services/auth-service/src/main.rs

# Build dependencies
WORKDIR /app/services/auth-service
RUN cargo build --release && rm -rf src

# Copy source dan build
COPY services/auth-service/src ./src
RUN touch src/main.rs && cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 libpq5 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/services/auth-service/target/release/auth-service /app/auth-service

EXPOSE 3001
ENV RUST_LOG=info

CMD ["/app/auth-service"]
