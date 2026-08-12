#!/usr/bin/env python3
import json
import re
import sys
from pathlib import Path


def read_version(path, pattern):
    content = Path(path).read_text()
    match = re.search(pattern, content, re.MULTILINE)
    if not match:
        raise ValueError(f"no version found in {path}")
    return match.group(1)


def main():
    if len(sys.argv) != 2:
        raise SystemExit("usage: check-release-versions.py VERSION")

    expected = sys.argv[1].removeprefix("v")
    components = {
        "VS Code Extension": ("editor/vscode/package.json", lambda path: json.loads(path.read_text())["version"]),
        "CLI": ("compiler/crates/cli/Cargo.toml", lambda path: read_version(path, r'^version\s*=\s*"([^"]+)"$')),
        "LSP": ("compiler/crates/lsp/Cargo.toml", lambda path: read_version(path, r'^version\s*=\s*"([^"]+)"$')),
        "Zed Cargo package": ("editor/zed/Cargo.toml", lambda path: read_version(path, r'^version\s*=\s*"([^"]+)"$')),
        "Zed extension": ("editor/zed/extension.toml", lambda path: read_version(path, r'^version\s*=\s*"([^"]+)"$')),
        "JetBrains plugin": (
            "editor/jetbrains/lsp-plugin/build.gradle.kts",
            lambda path: read_version(path, r'^\s*version\s*=\s*"([^"]+)"$'),
        ),
    }

    mismatches = []
    for name, (filename, reader) in components.items():
        path = Path(filename)
        try:
            found = reader(path)
        except (OSError, ValueError, KeyError) as error:
            mismatches.append((name, filename, str(error)))
            continue
        print(f"{name}: {found}")
        if found != expected:
            mismatches.append((name, filename, found))

    if mismatches:
        print(f"Expected: {expected}")
        for name, filename, found in mismatches:
            print(f"Mismatch: {name} ({filename}): {found}")
        return 1

    print(f"All release component versions match {expected}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
