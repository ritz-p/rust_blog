# Static export update timestamps

`docker/export-articles.sh` enables `RUST_BLOG_FILE_TIMESTAMPS=1` for seed. Articles and fixed pages use the last Git commit that changed their Markdown file as `updated_at`, instead of the time the temporary export database was created. Front matter changes (including tags and categories) count as file changes. Unrelated commits and checkout/mtime changes do not change this timestamp.

Uncommitted changes and files outside Git use the file's modification time, rounded to seconds. Preserve source file modification times when copying content outside Git. A fresh checkout of committed content uses Git history, so exports on different machines remain consistent without a persistent database or timestamp cache.

Fetch the complete history (`actions/checkout` with `fetch-depth: 0`). Shallow history is rejected to avoid silently assigning the shallow boundary commit date. Standalone bundle users can set `RUST_BLOG_FILE_TIMESTAMPS=1` when invoking seed; Git must be installed when the content is inside a Git repository.

Ordinary server seed keeps its existing change-based database behavior when the environment variable is absent. `created_at` remains the publication date; template, CSS and JavaScript changes do not change an article's `updated_at`.
