# https://just.systems

install-lsp:
    cargo install --path compiler/crates/lsp

test-workspace:
    cd compiler && cargo test --workspace --all-targets

build-workspace:
    cd compiler && cargo build --workspace --all-targets

clippy-workspace:
    cd compiler && cargo clippy --workspace --all-targets -- -D warnings

build-compiler:
    cd compiler && cargo build --release
