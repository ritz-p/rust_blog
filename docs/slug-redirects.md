# Redirects after changing a slug

Rename the slug in the article Markdown, then record the old slug in the repository's `redirects.toml`:

```toml
[articles]
"old-article" = "new-article"

[pages]
"old-about" = "about"
```

Set `[common].redirect_map_path` in `blog_config.toml` to use a different file. Paths are relative to the working directory. An absent default file means no redirects; an explicitly configured missing file is an error.

Values are slugs, not paths or external URLs. Chains are flattened to their final destination. Cycles, unsafe slugs, ambiguous source names, missing destinations and sources that still exist in the database are rejected before static output is cleared. When reusing a server database, remove the old article explicitly with `deleted: true` or migrate its slug before enabling the map.

The server returns HTTP 308 for mapped URLs. Static export prepends 308 rules to `_redirects` and creates small HTML redirect pages for hosts such as the Nginx preview that do not interpret `_redirects`. Both slash and non-slash URLs are supported on Cloudflare. Static targets end in `/`; server targets do not. Future articles are not redirect targets until their publication date.

The Markdown fixture in `tests/fixtures/redirects/renamed.md` is parsed and seeded by regression tests to verify encoded destinations, chained redirects and publication filtering.
