# Article metadata

Set `public_url = "https://blog.example.com"` in `[common]` of `blog_config.toml` to enable absolute canonical and Open Graph URLs. Use the public site root without a query or fragment.

Article pages include a plain-text description (excerpt, or the first 160 characters of the body), Open Graph title, description, site name and type. The article image falls back to `default_icatch_path`; relative image paths use the public URL. Without a public URL, canonical/og:url and relative og:image are omitted.

Server URLs use `/posts/slug`; static URLs use `/posts/slug/`. Slugs are URL encoded and metadata is HTML escaped. Re-export after changing configuration.
