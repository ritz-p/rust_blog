# Internal link checks

After exporting articles and recreating the static container, run:

```sh
docker compose exec -T -e STATIC_SITE_URL=http://static web node tests/frontend/check-links.cjs dist
```

The JavaScript workflow runs this check after export. It reads generated HTML through nginx and validates internal article/page links, images, stylesheets, scripts and HTML fragment targets against the exported file inventory. Relative URLs, URL-encoded slugs, queries and both `/posts/slug` and `/posts/slug/` forms are handled. Errors include the referring generated HTML file, URL and reason; all detected errors are reported before a nonzero exit. The file path identifies the rendered article by slug rather than its Markdown filename. External sites, mail links and data URLs are not fetched.

This verifies the static deployment and internal URLs that can be resolved to exported files; it does not replace server route tests or validate server-only dynamic endpoints. `checkLinks` can also read temporary HTML fixtures without HTTP in unit tests.

Deliberately broken fixtures and tracked rendering defects are listed in `tests/frontend/link-exceptions.json` with an exact source file, URL, failure reason and explanatory note. Exceptions are not wildcard exclusions. If a previously broken link is fixed or removed, the stale exception fails CI and must be removed too.
