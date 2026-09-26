# Synchronizing tags and categories

Markdown is the source of truth for each seeded article's `tags` and `categories`. Reseeding replaces the article's associations; an empty list or an omitted field clears them. Case variants still share the same normalized slug.

Each taxonomy update runs in a transaction. Failed updates preserve the previous associations. Entries used by other articles are retained; unreferenced tags and categories are removed. A successful full seed also prunes entries left behind by articles with `deleted: true`.

When seeding a Markdown article, its content, tag/category associations, taxonomy cleanup and file-based update timestamp share one outer transaction. Failure rolls back the entire article, including newly created tags and categories. Other files continue processing, and the CLI reports accumulated errors at the end. This is an article-level transaction, not a rollback of the whole batch.

Removing a Markdown file alone does not delete an article. Use `deleted: true` to explicitly delete it.
