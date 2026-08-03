# https://just.systems

install-lsp:
    cargo install --path compiler/crates/lsp

install-compiler:
    cargo install --path compiler/crates/cli

test-workspace:
    cd compiler && cargo test --workspace --all-targets

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
