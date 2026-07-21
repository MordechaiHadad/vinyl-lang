pub fn compile(
    source: &str,
) -> Result<Vec<vinyl_typecheck::hir::HirItem>, Vec<vinyl_typecheck::TypeError>> {
    let (result, _) = compile_with_warnings(source);
    result
}

pub fn compile_with_warnings(
    source: &str,
) -> (Result<Vec<vinyl_typecheck::hir::HirItem>, Vec<vinyl_typecheck::TypeError>>, Vec<vinyl_typecheck::CompileWarning>) {
    let tree = vinyl_parser::parse(source).unwrap();
    let items = vinyl_parser::lower::lower(&tree, source, "<test>").unwrap();
    let mut warnings = Vec::new();
    let result = vinyl_typecheck::typeck(&items, source, "<test>", &mut warnings);
    (result, warnings)
}
