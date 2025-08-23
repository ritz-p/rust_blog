FROM rust:latest

WORKDIR /workspace/rust_blog
RUN apt update && \
    apt install -y sqlite3
RUN cargo install sea-orm-cli
RUN rustup component add rustfmt
EXPOSE 8888

ENV RUST_BACKTRACE=1

RUN echo 'alias seaorm="sea-orm-cli"' >> ~/.bashrc