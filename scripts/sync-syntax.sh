#!/usr/bin/env bash
# Syncs the canonical syntax sources into editor consumers.
# Treesitter queries: editor/queries/vinyl/ -> nvim + zed.
# TextMate grammar:   editor/syntax/     -> vscode + jetbrains.
# Usage: sync-syntax.sh [--check] <all|nvim|zed|vscode|jetbrains>
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
check=0
if [[ "${1:-}" == "--check" ]]; then
  check=1
  shift
fi
consumer="${1:?usage: sync-syntax.sh [--check] <all|nvim|zed|vscode|jetbrains>}"
check_args=()
if [[ $check -eq 1 ]]; then
  check_args=(--check)
fi

case "$consumer" in
  all)
    for target in nvim zed vscode jetbrains; do
      "$0" "${check_args[@]}" "$target"
    done
    exit 0
    ;;
esac

# Each case maps one canonical source dir to a consumer's target dir.
case "$consumer" in
  nvim)
    target="$root/editor/nvim/queries/vinyl"
    source="$root/editor/queries/vinyl"
    files=(folds.scm highlights.scm indents.scm locals.scm textobjects.scm)
    ;;
  zed)
    target="$root/editor/zed/languages/vinyl"
    source="$root/editor/queries/vinyl"
    files=(highlights.scm)
    ;;
  vscode)
    target="$root/editor/vscode/syntaxes"
    source="$root/editor/syntax"
    files=(vinyl.tmLanguage.json)
    ;;
  jetbrains)
    target="$root/editor/jetbrains/vinyl.tmBundle/Syntaxes"
    source="$root/editor/syntax"
    files=(vinyl.tmLanguage.json)
    ;;
  *)
    echo "unknown consumer: $consumer" >&2
    exit 1
    ;;
esac

if [[ $check -eq 0 ]]; then
  mkdir -p "$target"
fi
for file in "${files[@]}"; do
  src="$source/$file"
  dst="$target/$file"
  if [[ $check -eq 1 ]]; then
    if ! cmp -s "$src" "$dst"; then
      echo "drift: $dst differs from $src" >&2
      exit 1
    fi
  else
    cp "$src" "$dst"
    echo "synced $dst"
  fi
done