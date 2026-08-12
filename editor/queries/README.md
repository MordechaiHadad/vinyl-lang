# Query Source Of Truth

Edit the files in `editor/queries/vinyl/`, not the generated consumer copies.

Run this from `vinyl-lang` after changing a query:

```sh
just sync-queries
```

The sync contract is:

- Neovim receives all five queries: `folds.scm`, `highlights.scm`, `indents.scm`, `locals.scm`, and `textobjects.scm`.
- Zed receives the shared `highlights.scm` query.
- Zed's `indents.scm` and `outline.scm` remain Zed-specific because their capture conventions differ from Neovim's.

Before committing, run:

```sh
just check-queries
```

This fails if any consumer copy drifts from the canonical files.
