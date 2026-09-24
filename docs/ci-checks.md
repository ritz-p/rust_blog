# Markdown and JavaScript CI

Both checks use `docker-compose.ci.yml` with a dedicated CI image. The image includes Rust, Node and SQLite, but does not install interactive development tools. BuildKit caches its layers in Actions.

The shared `prepare-checks` action caches four binaries: `format_markdown`, `migration`, `seed` and `export`. An exact cache key covers Rust sources, embedded assets/templates, manifests, the lockfile, toolchain and build scripts. There is no fallback binary key: changed build inputs force a rebuild. Markdown-only changes reuse the binaries and still validate/export the current checkout. Rust unit tests, including formatter tests, remain in Cargo Test.

For a local equivalent:

```sh
export COMPOSE_FILE=docker-compose.yml:docker-compose.ci.yml
docker compose up -d --build web
docker compose exec web sh docker/build-check-tools.sh
docker compose exec web .ci-tools/format_markdown --check content/articles
docker compose exec -e RUST_BLOG_TOOLS_DIR=.ci-tools web sh docker/export-articles.sh
```

The `.ci-tools` directory is generated and ignored by Git. `RUST_BLOG_TOOLS_DIR` is optional; without it, the export script builds and runs Cargo as before. Cache misses are expected on the first run and after Rust changes. Inspect Actions step timings to measure cold and warm runs separately.
