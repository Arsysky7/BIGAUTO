FROM rust:1.90-slim as builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace files
COPY Cargo.toml ./
COPY shared ./shared

# Copy service files
COPY services/user-service/Cargo.toml ./services/user-service/
RUN mkdir -p services/user-service/src && echo "fn main() {}" > services/user-service/src/main.rs

# Build dependencies
WORKDIR /app/services/user-service
RUN cargo build --release && rm -rf src

# Copy source dan build
COPY services/user-service/src ./src
RUN touch src/main.rs && cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 libpq5 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/services/user-service/target/release/user-service /app/user-service

EXPOSE 3002
ENV RUST_LOG=info

CMD ["/app/user-service"]
