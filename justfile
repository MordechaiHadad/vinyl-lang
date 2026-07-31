# https://just.systems

install-lsp:
    cargo install --path compiler/crates/lsp

test-compiler:
    cd compiler && cargo test --workspace --all-targets
