# Scheduled static publication

Set an article's `created_at` (or `date`) to its publication time:

```yaml
created_at: "2026-10-01T09:00:00+09:00"
```

RFC3339 offsets are respected. A datetime without an offset is interpreted as JST; a date alone means midnight JST. Static export requires an explicit article date.

Seed stores future articles in the database. Export excludes them from article pages, lists, archives, navigation and the search index until the publication time. A static site cannot change by itself when that time arrives: export and deployment must run again.

## GitHub Actions

The `Export articles and deploy` workflow checks every 15 minutes (at minutes 7, 22, 37 and 52 UTC). To enable scheduled publication after merging into the default branch:

1. Configure `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` secrets and the existing Wrangler deployment configuration.
2. Set the repository Actions variable `STATIC_SCHEDULE_ENABLED` to `true`.
3. Commit the scheduled article to the default branch.

Each run seeds a fresh database from Markdown, exports the currently published articles, and deploys only after successful export. Setting the variable to `false` disables scheduled runs. Manual runs retain the existing `deploy` checkbox, which defaults to false.

Publication happens on the next successful deployment after the article date, not at an exact second. GitHub Actions can delay scheduled runs; build and deployment time are additional. See [GitHub's scheduling guidance](https://docs.github.com/en/actions/how-tos/troubleshoot-workflows). Scheduled workflows run on the default branch. No scheduled deployment is enabled merely by pushing the feature branch.

## Separate content repositories and other hosts

Install the export bundle in the repository that owns the Markdown and configure a scheduler there to run migration, seed, export and deployment in that order. Alternatively, run `docker/export-articles.sh` followed by your deployment command from cron with the repository as working directory. Rebuild from Markdown to reflect edited dates, deleted articles and cancellations.

Future article contents are retained in the database artifact, so treat that artifact as private. Do not publish it with `dist/`. Images are copied as assets irrespective of article dates; do not place embargoed files in the public asset directory.
