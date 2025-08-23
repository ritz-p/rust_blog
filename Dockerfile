FROM rust:latest

WORKDIR /workspace/rust_blog
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    sqlite3 mold clang pkg-config && \
    rm -rf /var/lib/apt/lists/*
RUN cargo install sea-orm-cli
RUN rustup component add rustfmt
EXPOSE 8888

ENV RUST_BACKTRACE=1

RUN echo 'alias seaorm="sea-orm-cli"' >> ~/.bashrc