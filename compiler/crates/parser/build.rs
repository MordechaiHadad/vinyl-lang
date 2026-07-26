fn main() {
    let source = std::path::Path::new("../../../grammar/src");
    cc::Build::new()
        .include(source)
        .file(source.join("parser.c"))
        .compile("tree-sitter-vinyl");
    println!("cargo:rerun-if-changed=../../../grammar/src/parser.c");
}
