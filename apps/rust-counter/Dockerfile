FROM rust:1.87-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.toml
COPY src src
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/rust-counter /usr/local/bin/rust-counter

EXPOSE 8080
CMD ["rust-counter"]
