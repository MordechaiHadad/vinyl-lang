# Vinyl for Zed

Language support for the **Vinyl** programming language in [Zed](https://zed.dev).

## Features

- Tree-sitter syntax highlighting, outlining, and indentation for `.vn` files.
- LSP integration (diagnostics, completion, go-to-definition, hover) via `vinyl-lsp`.

## LSP binary resolution

1. `vinyl-lsp` on `$PATH`.
2. Otherwise, the latest `vinyl-lsp` build is downloaded from the
   [GitHub releases](https://github.com/MordechaiHadad/vinyl-lang/releases)
   of the vinyl-lang repository.

## Building

```sh
cargo build --release --target wasm32-wasip2
```

The build target requires the `wasm32-wasip2` Rust target (not `wasm32-wastip1`):

```sh
rustup target add wasm32-wasip2
```

Load the extension from this directory via `zed: install local extension`.

## License

MIT