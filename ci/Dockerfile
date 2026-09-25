FROM rust:1.98.1-bookworm

WORKDIR /workspace/rust_blog
RUN apt-get update && apt-get install -y --no-install-recommends \
    sqlite3 mold clang pkg-config ca-certificates gnupg && \
    curl -fsSL https://deb.nodesource.com/setup_24.x | bash - && \
    apt-get install -y --no-install-recommends nodejs && \
    apt-get clean && rm -rf /var/lib/apt/lists/*
COPY rust-toolchain.toml ./
RUN rustup show active-toolchain
ENV CARGO_TARGET_DIR=/tmp/rust-blog-target
CMD ["sleep", "infinity"]
