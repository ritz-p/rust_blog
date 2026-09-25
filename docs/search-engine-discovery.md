# Sitemap and robots.txt

Configure the public origin in `blog_config.toml`:

```toml
[common]
public_url = "https://blog.example.com"
```

The server serves `/sitemap.xml` as XML and `/robots.txt` as plain text. Static export writes both files. Sitemap URLs use absolute public URLs, percent-encoded slugs, and the appropriate server/static trailing-slash convention. Article and fixed-page `lastmod` values come from database `updated_at`, not export time.

The sitemap includes the homepage, published articles, fixed pages, tag/category indexes and their primary detail pages. Future articles, search queries and alternate sort/pagination URLs are excluded. Redirect aliases are not content records and are not listed. The output is sorted and deduplicated.

Without `public_url`, robots.txt allows crawling but has no Sitemap line; sitemap.xml is omitted (404 in server mode). Only an HTTP(S) origin is accepted: credentials, path prefixes, queries and fragments are rejected. Static export validates this before replacing existing output. The `sitemap.xml` and `robots.txt` names are reserved for fixed-page slugs.

Regenerate and deploy the static site after publication or content changes. A sitemap with more than 50,000 URLs or 50 MB is rejected and requires splitting into multiple sitemaps.
