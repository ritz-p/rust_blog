FROM rust:latest

WORKDIR /workspace/rust_blog

RUN cargo install sea-orm-cli
RUN rustup component add rustfmt
EXPOSE 8888

ENV RUST_BACKTRACE=1