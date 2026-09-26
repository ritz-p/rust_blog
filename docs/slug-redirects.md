# Redirects after changing a slug

Rename the slug in the article Markdown, then record the old slug in the repository's `redirects.toml`:

```toml
[articles]
"old-article" = "new-article"

[pages]
"old-about" = "about"
```

Set `[common].redirect_map_path` in `blog_config.toml` to use a different file. Paths are relative to the working directory. An absent default file means no redirects; an explicitly configured missing file is an error.

The production image includes the repository's `redirects.toml` at `/app/redirects.toml` for both `server` and `export`. Rebuild the image after editing this file, or mount a replacement at that path.

Values are slugs, not paths or external URLs. Chains are flattened to their final destination. Cycles, unsafe slugs, ambiguous source names, missing destinations and sources that still exist in the database are rejected before static output is cleared. When reusing a server database, remove the old article explicitly with `deleted: true` or migrate its slug before enabling the map.

The server returns HTTP 308 for mapped URLs. Static export prepends 308 rules to `_redirects` and creates small HTML redirect pages for hosts such as the Nginx preview that do not interpret `_redirects`. Both slash and non-slash URLs are supported on Cloudflare. Static targets end in `/`; server targets do not. Future articles are not redirect targets until their publication date.

The Markdown fixture in `content/articles/32.md` is parsed and seeded by regression tests to verify encoded destinations, chained redirects and publication filtering.

Test aliases live in `tests/fixtures/redirects/redirects.toml`; the production map is empty by default. To preview these aliases locally, seed `content/articles`, then set `redirect_map_path = "tests/fixtures/redirects/redirects.toml"` under `[common]` in your local `blog_config.toml`. Restart the server, or rerun export and `docker compose up -d --force-recreate static`. Remove this local setting before deploying: production startup runs migrations but does not seed the test articles.
