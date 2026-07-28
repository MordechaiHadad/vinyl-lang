pub fn compile(
    source: &str,
) -> Result<Vec<vinyl_typecheck::hir::HirItem>, Vec<vinyl_typecheck::TypeDiagnostic>> {
    let (result, _) = compile_with_warnings(source);
    result
}

pub fn compile_with_warnings(
    source: &str,
) -> (
    Result<Vec<vinyl_typecheck::hir::HirItem>, Vec<vinyl_typecheck::TypeDiagnostic>>,
    Vec<vinyl_typecheck::TypeDiagnostic>,
) {
    let items = vinyl_parser::parse_and_lower(source).unwrap();
    let result = vinyl_typecheck::typeck(&items, source, "<test>");
    match result {
        Ok((hir, warnings)) => (Ok(hir), warnings),
        Err(errors) => (Err(errors), Vec::new()),
    }
}
