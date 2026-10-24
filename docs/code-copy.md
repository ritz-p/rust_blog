# Code block controls

Article and fixed-page code blocks display the fenced language token (or `Text` when none is specified) and a keyboard-accessible copy button. The controls sit outside `pre > code`, so copying uses only the code's text, including indentation, line breaks and Unicode characters. Highlighting spans and toolbar labels are never copied.

Clipboard success is announced through a status region. If clipboard access is unavailable or denied, the code is selected and the status asks the reader to copy it manually. Unknown languages retain their label without claiming syntax highlighting. Server rendering and static export share the language metadata and versioned navigation script. Re-export static sites to update the generated HTML and assets.
