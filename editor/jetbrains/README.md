# Vinyl for JetBrains IDEs

Language support for the **Vinyl** programming language in JetBrains IDEs,
built on the platform's native **LSP client API** (open-sourced in IntelliJ
Platform 2026.2; `LspIntegrationProvider` since 2026.1.4).

## Requirements

- IntelliJ IDEA Ultimate 2026.2+ (or another IntelliJ-based IDE of the same
  vintage with the LSP and Ultimate modules).

## Features

- TextMate syntax highlighting for `.vn` files via the bundled
  `vinyl.tmBundle`.
- LSP integration (completion, diagnostics, go-to-definition, hover) via
  `vinyl-lsp`.

## LSP binary resolution

1. `vinyl.lsp.path` system property (custom binary path).
2. `vinyl-lsp` on `$PATH`.
3. Otherwise, the latest `vinyl-lsp` build is downloaded from the
   [GitHub releases](https://github.com/MordechaiHadad/vinyl-lang/releases)
   and cached under the IDE system directory (`<system>/vinyl-lsp/<tag>`).

## Building

Requires JDK 21 and Gradle 8.x+:

```sh
cd editor/jetbrains/lsp-plugin
gradle buildPlugin
```

From the repository root, use `gradle -p editor/jetbrains/lsp-plugin buildPlugin`.

The distribution zip lands in `lsp-plugin/build/distributions/` and includes
the TextMate bundle. Install it from disk via
`Settings > Plugins > Install Plugin from Disk...`.

## Notes

- Syntax highlighting is provided by the TextMate Bundles plugin (bundled and
  enabled by default). If the grammar is not picked up automatically, add the
  `vinyl.tmBundle` directory once under `Editor > TextMate Bundles`.
- Community Edition (IC) is not supported: the LSP client requires the
  Ultimate and LSP modules.
- Legacy API: plugins targeting IDE versions before 2026.1.4 use the old
  `LspServerSupportProvider` / `LspServerManager` API. This plugin targets the
  open LSP client API only.

## License

MIT
