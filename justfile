# https://just.systems

# Installing stuff

install-lsp:
    cargo install --path compiler/crates/lsp

install-compiler:
    cargo install --path compiler/crates/cli

build-workspace:
    cd compiler && cargo build --workspace --all-targets

clippy-workspace:
    cd compiler && cargo clippy --workspace --all-targets -- -D warnings

build-compiler:
    cd compiler && cargo build --release

audit:
    cd compiler && cargo audit

audit-ci:
    cd compiler && cargo audit --deny warnings

# Test stuff

test-all: test-workspace test-grammar

test-workspace:
    cd compiler && cargo test --workspace --all-targets

test-grammar:
    cd grammar && tree-sitter test
