#!/bin/sh
set -eu

cd /workspace/rust_blog

# Build from the checked-in Markdown, independently of the development DB.
export_db_dir=$(mktemp -d /tmp/rust-blog-export.XXXXXX)
trap 'rm -f "$export_db_dir/blog.db" "$export_db_dir/blog.db-shm" "$export_db_dir/blog.db-wal"; rmdir "$export_db_dir"' EXIT
export DATABASE_URL="sqlite://$export_db_dir/blog.db?mode=rwc"
export ARTICLE_PATH=/workspace/rust_blog/content/articles
export FIXED_CONTENT_PATH=/workspace/rust_blog/content/fixed_contents
export CONFIG_TOML_PATH=/workspace/rust_blog/blog_config.toml
export RUST_BLOG_CONFIG_PATH="$CONFIG_TOML_PATH"
export RUST_BLOG_CONTENT_DIR=/workspace/rust_blog/content
export RUST_BLOG_TEMPLATES_DIR=/workspace/rust_blog/templates

cargo run --locked -p migration --bin migration -- up
cargo run --locked -p rust_blog --bin seed
cargo run --locked -p rust_blog --bin export -- dist
