FROM rust:1.88 AS builder

WORKDIR /app

# Copy the entire workspace
COPY . .

# Build the release binary
RUN cargo build --release --bin executor

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary and migrations from builder
COPY --from=builder /app/target/release/executor /app/executor
COPY --from=builder /app/crates/db/migrations /app/migrations

ENV MIGRATIONS_PATH=/app/migrations

CMD ["./executor"]
