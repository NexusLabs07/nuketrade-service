FROM rust:1.88 AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock rust-toolchain.toml rustfmt.toml ./
COPY crates ./crates
COPY bin ./bin

RUN cargo build --locked --release --bin executor

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd --system app \
    && useradd --system --gid app --create-home app

WORKDIR /app

COPY --from=builder --chown=app:app /app/target/release/executor /app/executor
COPY --from=builder --chown=app:app /app/crates/db/migrations /app/migrations

ENV MIGRATIONS_PATH=/app/migrations

USER app

CMD ["./executor"]