pub fn compile(
    source: &str,
) -> Result<Vec<vinyl_typecheck::hir::HirItem>, Vec<vinyl_typecheck::TypeError>> {
    let tree = vinyl_parser::parse(source).unwrap();
    let items = vinyl_parser::lower::lower(&tree, source, "<test>").unwrap();
    vinyl_typecheck::typeck(&items, source, "<test>")
}
