# Footnotes

Markdown footnotes receive generated `footnote-N` targets and unique `footnote-ref-N-M` reference IDs. Each reference links to the note, and each note includes a separately labelled return link for every reference. The renderer is shared by server pages and static export.

Only generated markup receives these IDs after sanitization; arbitrary source HTML IDs and scripts remain filtered. `content/articles/46.md` exercises a repeated Japanese footnote. If issue #192's link checker is merged, remove the three obsolete `missing fragment` exceptions for articles 10 and 46; the checker intentionally rejects unused exceptions.
