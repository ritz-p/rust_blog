# Scheduled static publication

Set an article's `created_at` (or `date`) to its publication time:

```yaml
created_at: "2026-10-01T09:00:00+09:00"
```

RFC3339 offsets are respected. A datetime without an offset uses `[common].timezone` (UTC by default), and a date alone means midnight in that timezone. Specify an explicit offset for local times repeated or skipped during a timezone transition. Static export requires an explicit article date.

Export writes the article page and search data even before the publication time. Anyone with the article URL can read it after deployment. Opening or reloading a page with JavaScript at or after the scheduled time includes the article in lists, search and sidebar links using the browser clock. Without JavaScript, the lists show the export-time snapshot. Atom and the sitemap reflect the next export after the publication time.

## GitHub Actions

The `Export articles and deploy` workflow runs weekly on Mondays at 09:07 JST (00:07 UTC). To enable scheduled publication after merging into the default branch:

1. Configure `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` secrets and the existing Wrangler deployment configuration.
2. Set the repository Actions variable `STATIC_SCHEDULE_ENABLED` to `true`.
3. Commit the scheduled article to the default branch.

Each run seeds a fresh database from Markdown, exports article pages and refreshes Atom and the sitemap, and deploys only after successful export. Setting the variable to `false` disables scheduled runs. Manual runs retain the existing `deploy` checkbox, which defaults to false.

Both the static site and database artifacts are retained for 7 days, including manual runs, to limit Actions artifact storage usage.

The weekly deployment refreshes Atom and the sitemap. GitHub Actions can delay scheduled runs. Scheduled workflows run on the default branch.

## Separate content repositories and other hosts

Install the export bundle in the repository that owns the Markdown and configure a scheduler there to run migration, seed, export and deployment in that order. When using this repository's development environment, run `docker compose exec -T web sh docker/export-articles.sh` followed by your deployment command from cron with the repository as working directory and `web` already running. Rebuild from Markdown to reflect edited dates, deleted articles and cancellations.

The deployed HTML, images and search data are public as soon as they are deployed. Keep unpublished drafts outside the export inputs. Keep the database artifact outside `dist/`.
