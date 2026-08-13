# vinyl.nvim

Neovim support for the [Vinyl](https://github.com/MordechaiHadad/vinyl-lang)
programming language: tree-sitter highlighting, folds and text objects, and
`vinyl-lsp` integration.

## Requirements

- Neovim 0.10+ with tree-sitter.
- `tree-sitter` CLI to build the parser (`build_parser` step below).
- `vinyl-lsp`: downloaded from GitHub Releases on first use, or built via Cargo.

## Install

With [lazy.nvim](https://github.com/folke/lazy.nvim):

```lua
{
  "MordechaiHadad/vinyl.nvim",
  ft = { "vinyl" },
  build = function()
    require("vinyl").build_parser()
  end,
}
```

On first load the plugin registers the `vinyl` filetype, starts tree-sitter
highlighting, and, if `vinyl-lsp` is not on `$PATH`, prompts for how to obtain
it (prebuilt release or Cargo build).

## Sources

This plugin is maintained inside the `vinyl-lang` monorepo under
`editor/nvim/` and published to this repository on each release. The
tree-sitter queries and TextMate grammar shared with the other editor
extensions live in the monorepo; do not submit edits here directly.

## License

MIT