#!/bin/sh
set -eu

case "${1:-sleep}" in
  sleep)
    exec sleep infinity
    ;;
  sh|/bin/sh)
    exec /bin/sh
    ;;
  bash|/bin/bash)
    exec /bin/bash
    ;;
  *)
    exec "$@"
    ;;
esac
