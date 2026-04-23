# rust-counter

A minimal Rust HTTP service for the test endpoint at `rust-counter.yral.com`.

## Behavior

- `GET /` increments a counter stored in PostgreSQL and returns the new value as JSON:
  - `{ "value": 1 }`
- `GET /healthz` returns `ok`

## Required environment variables

- `DATABASE_URL` (PostgreSQL connection string)
- Optional: `PORT` (defaults to `8080`)

## Run locally

```bash
cargo run
```

## Build container image

```bash
docker build -t ghcr.io/dolr-ai/rust-counter:v0.1.0 .
```
