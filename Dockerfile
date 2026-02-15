FROM rust:latest

WORKDIR /workspace/rust_blog
RUN useradd -m -s /bin/bash -u 1000 vscode && mkdir -p /home/vscode && chown -R 1000:1000 /home/vscode
ENV CARGO_HOME=/home/vscode/.cargo
ENV RUSTUP_HOME=/home/vscode/.rustup
ENV CARGO_TARGET_DIR=/home/vscode/.cargo/target
RUN mkdir -p /home/vscode/.cargo /home/vscode/.cargo/target /home/vscode/.rustup && chown -R 1000:1000 /home/vscode/.cargo /home/vscode/.rustup
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    sqlite3 mold clang pkg-config git ca-certificates gnupg && \
    curl -fsSL https://deb.nodesource.com/setup_24.x | bash - && \
    apt-get install -y --no-install-recommends nodejs && \
    npm i -g @openai/codex && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

ARG USERNAME=vscode
ARG USER_UID=1000
ARG USER_GID=1000
RUN groupadd --gid ${USER_GID} ${USERNAME} && \
    useradd --uid ${USER_UID} --gid ${USER_GID} -m ${USERNAME}
USER ${USERNAME}
ENV PATH=/home/${USERNAME}/.cargo/bin:$PATH
RUN rustup default stable
RUN cargo install sea-orm-cli
RUN rustup component add rustfmt



EXPOSE 8888

ENV RUST_BACKTRACE=1
RUN echo 'alias seaorm="sea-orm-cli"' >> /etc/bash.bashrc
USER ${USERNAME}
