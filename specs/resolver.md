# Resolver

The resolver has two modes determined by whether a `vinyl.toml` manifest file is found.

## Mode Detection

Walk up parent directories from the entry point looking for `vinyl.toml`. If found, enter **manifest mode** with the project root set to that directory. If parent is `None` (filesystem root reached) without finding `vinyl.toml`, enter **script mode** with the project root anchored to the entry file's directory.

## Import Prefix Mapping

| Prefix | Maps to | Manifest | Script |
|--------|---------|----------|--------|
| `parent::x` | `./x` | Yes | Yes |
| `parent::parent::x` | `../x` | Yes | Yes |
| `parent::parent::parent::x` | `../../x` | Yes | Yes |
| `package::x` | `{root}/x.vn` | Yes | No |
| `self::x` | error (refers to current file, not an external module) | — | — |

`self` refers to the current file itself — `self::` is not valid in import statements. Use `parent::` for same-directory relative imports.

`parent::` is stackable. Each `parent::` goes up one additional directory level from the current file's parent. The compiler warns when 4 or more `parent::` levels are used, suggesting the user switch to a manifest-based project.

## Manifest Mode

- Requires a `src/` directory under the project root — all source files live under `src/`.
- **Eager resolution**: Walks all `*.vn` files under `src/` at startup and registers them as modules.
- Respects `.gitignore` rules — ignored paths are excluded from module discovery.
- Additional ignore rules may be defined in the future (none specified yet).
- Imports use `parent::`, `package::` or no prefix.

## Script Mode

- No `vinyl.toml` found; the file's directory becomes the project root.
- Entry can be implicit (`main.vn` in the project root) or an explicit file path passed by the user.
- **Lazy resolution**: Only imports referenced by the entry file (transitively) are resolved — other files are ignored.
- `package::` is not available. Only `parent::` (with no prefix) is valid.
- Respects `.gitignore` rules.

## LSP Integration

The LSP operates in script mode semantics. It anchors to the currently opened file and lazy-resolves its imports. Files registered via the LSP's virtual file system (VFS) are eligible for auto-import resolution.

## Workspaces

Placeholder — workspace/ multi-root support is not yet designed.

# Editions

Vinyl uses an edition system (like Rust's) to allow syntax and semantics to evolve without breaking existing code.

- Single-file scripts use the latest stable edition automatically.
- Projects with a `vinyl.toml` config file declare the edition in that file (e.g. `edition = "2025"`). The package manager or user sets this; no default fallback.
- The default edition is the latest stable edition.
- Editions can change grammar rules, keywords, operator precedence, and standard library behavior.
- The compiler always supports at least the two most recent editions.