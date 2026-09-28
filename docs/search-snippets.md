# Search result snippets

Search results highlight matching terms in titles and show up to 160 Unicode characters around the first matching body term, with up to 40 characters of preceding context. Ellipses indicate omitted text. Multiple terms are highlighted within the selected snippet; a single snippet does not necessarily include every matching term.

If the body has no match (for example, only the title matches), results use the normal excerpt and highlight any matching terms there. Server templates escape each text fragment before wrapping matched fragments in `mark`. Static search builds text nodes and `mark` elements, never HTML from article strings. The search index now includes the original plain-text body so capitalization is preserved. Re-export the site after updating; the search script URL is versioned to avoid reusing the previous cached script.

Search matching remains case-insensitive substring matching with whitespace-separated AND terms; this change does not add stemming, fuzzy matching or Unicode normalization.
