# Dockerfile for Docker Swarm deployment
FROM alpine:3.19

# Install ca-certificates and wget for healthcheck
RUN apk add --no-cache ca-certificates wget

WORKDIR /app

# Copy the binary and site assets
COPY ./target/x86_64-unknown-linux-musl/release/yral-auth .
COPY ./target/x86_64-unknown-linux-musl/release/hash.txt .
COPY ./target/site ./site

# Copy entrypoint script
COPY ./deploy/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

# Set Leptos environment variables (production runtime config)
ENV LEPTOS_OUTPUT_NAME="yral-auth"
ENV LEPTOS_SITE_ROOT="site"
ENV LEPTOS_SITE_PKG_DIR="pkg"
ENV LEPTOS_SITE_ADDR="0.0.0.0:8080"
ENV LEPTOS_ENV="PROD"
ENV LEPTOS_HASH_FILES="true"
ENV APP_ENV="production"

EXPOSE 8080

ENTRYPOINT ["/entrypoint.sh"]
