# CI

Cargo Fmt, Clippy, Cargo Test, Markdown and JavaScript checks target pull requests to `develop`. Markdown and JavaScript also run on pushes to `develop`. Cargo Test runs only when its Rust build inputs, CI container configuration, shared preparation action or workflow change; see [the testing guide](testing.md).

These checks and the release/export-tool builds run Rust commands in the `web` service using `docker-compose.ci.yml`. The dedicated CI image includes Rust, Node and SQLite, without interactive development tools. The shared `prepare-checks` action builds and starts this container. BuildKit caches its layers in Actions.

Markdown and JavaScript use the action's default `build-tools: "true"` to cache four binaries: `format_markdown`, `migration`, `seed` and `export`. An exact cache key covers Rust sources, embedded assets/templates, manifests, the lockfile, toolchain and build scripts. There is no fallback binary key: changed build inputs force a rebuild. Markdown-only changes reuse the binaries and still validate/export the current checkout.

Cargo Fmt, Clippy, Cargo Test and release/export-tool builds set `build-tools: "false"` to skip the helper cache and compilation. Rust unit tests, including formatter tests, remain in Cargo Test. Release workflows copy binaries from the container's `/tmp/rust-blog-target` directory before uploading artifacts. Release and export-tool builds run on `v*` tags; the export-tool build also supports manual runs.

For a local equivalent of the Markdown/export checks:

```sh
export COMPOSE_FILE=docker-compose.yml:docker-compose.ci.yml
docker compose up -d --build web
docker compose exec web sh docker/build-check-tools.sh
docker compose exec web .ci-tools/format_markdown --check content/articles
docker compose exec -e RUST_BLOG_TOOLS_DIR=.ci-tools web sh docker/export-articles.sh
```

The `.ci-tools` directory is generated and ignored by Git. `RUST_BLOG_TOOLS_DIR` is optional; without it, the export script builds and runs Cargo as before. Cache misses are expected on the first run and after Rust changes. Inspect Actions step timings to measure cold and warm runs separately.
