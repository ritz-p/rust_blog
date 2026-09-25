#!/bin/sh
set -eu
cargo build --locked -p rust_blog --bin format_markdown --bin seed --bin export
cargo build --locked -p migration --bin migration
mkdir -p .ci-tools
for binary in format_markdown seed export migration; do
    cp "${CARGO_TARGET_DIR:?}/debug/$binary" ".ci-tools/$binary"
    strip ".ci-tools/$binary"
done
