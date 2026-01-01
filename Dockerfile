FROM rust:latest

WORKDIR /workspace/rust_blog
RUN useradd -m -s /bin/bash -u 1000 vscode && mkdir -p /home/vscode && chown -R 1000:1000 /home/vscode
ENV CARGO_HOME=/home/vscode/.cargo
ENV RUSTUP_HOME=/home/vscode/.rustup
ENV CARGO_TARGET_DIR=/home/vscode/.cargo/target
RUN mkdir -p /home/vscode/.cargo /home/vscode/.cargo/target /home/vscode/.rustup && chown -R 1000:1000 /home/vscode/.cargo /home/vscode/.rustup
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    sqlite3 mold clang pkg-config git && \
    rm -rf /var/lib/apt/lists/*
USER vscode
RUN rustup default stable
RUN cargo install sea-orm-cli
RUN rustup component add rustfmt
EXPOSE 8888

ENV RUST_BACKTRACE=1
RUN echo 'alias seaorm="sea-orm-cli"' >> ~/.bashrc

