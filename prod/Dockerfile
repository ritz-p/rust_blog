FROM rust:latest AS builder

WORKDIR /app

RUN apt-get update && \
    apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY core/Cargo.toml core/Cargo.toml
COPY migration/Cargo.toml migration/Cargo.toml
COPY Rocket.toml Rocket.toml
COPY blog_config.toml blog_config.toml
COPY core/src core/src
COPY core/assets core/assets
COPY templates templates
COPY migration/src migration/src
COPY content/image content/image
COPY content/icon content/icon

RUN cargo build --release -p rust_blog -p migration --bin rust_blog

FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && \
    apt-get install -y --no-install-recommends sqlite3 ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/rust_blog /app/rust_blog
COPY --from=builder /app/target/release/migration /app/migration
COPY --from=builder /app/templates /app/templates
COPY --from=builder /app/content /app/content
COPY --from=builder /app/blog_config.toml /app/blog_config.toml
COPY --from=builder /app/Rocket.toml /app/Rocket.toml
COPY prod/entrypoint.sh /app/entrypoint.sh

RUN chmod +x /app/entrypoint.sh && \
    mkdir -p /data

ENV ROCKET_ENV=release
ENV PORT=8080
ENV DB_PATH=/data/blog.db

EXPOSE 8080

ENTRYPOINT ["/app/entrypoint.sh"]
