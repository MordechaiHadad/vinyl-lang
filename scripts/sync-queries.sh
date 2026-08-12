#!/usr/bin/env bash
# Syncs the canonical Tree-sitter query pool in editor/queries/vinyl/
# into an editor consumer's query directory.
# Usage: sync-queries.sh [--check] <all|nvim|zed>
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
pool="$root/editor/queries/vinyl"
check=0
if [[ "${1:-}" == "--check" ]]; then
  check=1
  shift
fi
consumer="${1:?usage: sync-queries.sh [--check] <all|nvim|zed>}"
check_args=()
if [[ $check -eq 1 ]]; then
  check_args=(--check)
fi

case "$consumer" in
  all)
    "$0" "${check_args[@]}" nvim
    "$0" "${check_args[@]}" zed
    exit 0
    ;;
  nvim)
    target="$root/../vinyl.nvim/queries/vinyl"
    queries=(folds highlights indents locals textobjects)
    ;;
  zed)
    target="$root/editor/zed/languages/vinyl"
    queries=(highlights)
    ;;
  *)
    echo "unknown consumer: $consumer" >&2
    exit 1
    ;;
esac

if [[ $check -eq 0 ]]; then
  mkdir -p "$target"
fi
for query in "${queries[@]}"; do
  src="$pool/$query.scm"
  dst="$target/$query.scm"
  if [[ $check -eq 1 ]]; then
    if ! cmp -s "$src" "$dst"; then
      echo "drift: $dst differs from $pool/$query.scm" >&2
      exit 1
    fi
  else
    cp "$src" "$dst"
    echo "synced $dst"
  fi
done
