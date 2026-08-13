# Syntax Source Of Truth

Canonical syntax sources (edit these, never the generated copies):

- `editor/queries/vinyl/` — treesitter queries (`folds.scm`, `highlights.scm`, `indents.scm`, `locals.scm`, `textobjects.scm`).
- `editor/syntax/vinyl.tmLanguage.json` — the TextMate grammar, shared by the VS Code and JetBrains extensions.

Run this from `vinyl-lang` after changing either:

```sh
just sync-syntax
```

This copies the canonical files into each consumer:

- Neovim receives all five queries at `editor/nvim/queries/vinyl/`.
- Zed receives the shared `highlights.scm` query at `editor/zed/languages/vinyl/`. Zed's `indents.scm` and `outline.scm` remain Zed-specific because their capture conventions differ from Neovim's.
- VS Code receives the TextMate grammar at `editor/vscode/syntaxes/`.
- JetBrains receives the TextMate grammar at `editor/jetbrains/vinyl.tmBundle/Syntaxes/`.

Before committing, run:

```sh
just check-syntax
```

This fails if any consumer copy drifts from the canonical files, and it also
runs `scripts/check-highlight-sync.py`, which enforces that the overlapping
lexical surface (keywords, primitive types, booleans, brackets, delimiters)
is identical between the treesitter sources and the TextMate grammar.