#!/bin/sh
set -eu

export_db_dir=$(mktemp -d /tmp/rust-blog-export.XXXXXX)
trap 'rm -f "$export_db_dir/blog.db" "$export_db_dir/blog.db-shm" "$export_db_dir/blog.db-wal"; rmdir "$export_db_dir"' EXIT
export DATABASE_URL="sqlite://$export_db_dir/blog.db?mode=rwc"
export CONFIG_TOML_PATH="${RUST_BLOG_CONFIG_PATH:-${CONFIG_TOML_PATH:-blog_config.toml}}"
export RUST_BLOG_CONFIG_PATH="$CONFIG_TOML_PATH"
export RUST_BLOG_REQUIRE_CREATED_AT=1

cargo run --locked -p migration --bin migration -- up
cargo run --locked -p rust_blog --bin seed
cargo run --locked -p rust_blog --bin export -- "${EXPORT_OUTPUT_DIR:-dist}"
