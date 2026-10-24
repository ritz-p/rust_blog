# Atom feed

Set `public_url = "https://your-domain.example"` under `[common]` in `blog_config.toml`. The site exposes an Atom 1.0 feed at `/feed.xml`; the home page and article pages advertise it using `rel="alternate"`. Static export writes the same feed and advertises it on generated pages. The nginx preview and Cloudflare headers use `application/atom+xml`.

The feed contains the newest 20 published articles by creation date, with deterministic slug ordering for ties. Future and deleted articles are excluded. Each entry includes its title, a plain-text summary of up to 300 characters, publication/update dates and an absolute article URL. Feed/entry timestamps come from article data rather than export time. An empty feed uses the Unix epoch as its deterministic update date.

Entry IDs use the canonical article URL without a trailing slash in both modes, so editing an article or switching output mode does not change its ID. Article links retain the correct server/static trailing-slash convention. Renaming a slug or changing `public_url` changes the entry identity; retain the existing URL when preserving subscription history matters.

Without `public_url`, the server returns 404 for `/feed.xml`, static export omits it and no discovery link is shown. Static feeds update only when export is rerun, including when scheduled articles become publishable. The fixed-page slug `feed.xml` is reserved. RSS is not separately generated; readers that support Atom can subscribe to this URL.
