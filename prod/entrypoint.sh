#!/bin/sh
set -eu

DB_PATH="${DB_PATH:-/data/blog.db}"
PORT="${PORT:-8080}"

mkdir -p "$(dirname "$DB_PATH")"

if [ ! -f "$DB_PATH" ]; then
  touch "$DB_PATH"
fi

export DATABASE_URL="${DATABASE_URL:-sqlite://$DB_PATH?mode=rwc}"
export ROCKET_ADDRESS="${ROCKET_ADDRESS:-0.0.0.0}"
export ROCKET_PORT="$PORT"
export ROCKET_ENV="${ROCKET_ENV:-release}"

echo "==> Running migrations"
/app/migration up

echo "==> Starting rust_blog on ${ROCKET_ADDRESS}:${ROCKET_PORT}"
exec /app/rust_blog
