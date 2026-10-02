# Article fixture regression tests

`cargo test -p rust_blog confirmation_articles` seeds selected real Markdown fixtures into an isolated in-memory database and requests the actual article route. It checks publication filtering, language highlighting, table-of-contents settings and minimal articles. It does not depend on the total article count.

The JavaScript CI exports all fixtures and runs `tests/frontend/articles.test.cjs` through nginx using `STATIC_SITE_URL`. It compares every rendered code block with its original Markdown content and checks images, TOC targets, empty content and future publication. Locally run `docker compose exec -T -e STATIC_SITE_URL=http://static web npm run test:js` after export and recreating the static container.

PowerShell highlighting (#194) and footnote fragment navigation (#193) are explicit TODO tests on the master baseline; their articles still render and their code/text remains covered. After merging those features, replace the TODO entries with strict assertions and remove the PowerShell exemption. Without `STATIC_SITE_URL`, the HTTP test is explicitly skipped rather than reported as a verified static deployment.
