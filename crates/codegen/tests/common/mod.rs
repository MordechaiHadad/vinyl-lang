use vinyl_codegen::CodegenBackend;
use vinyl_codegen::cranelift::CraneliftBackend;

pub fn run(source: &str) -> Result<i64, String> {
    let tree = vinyl_parser::parse(source).map_err(|_| "parse error")?;
    let items = vinyl_parser::lower::lower(&tree, source, "<test>")
        .map_err(|e| format!("lower error: {e:?}"))?;
    let mut warnings = Vec::new();
    let hir = vinyl_typecheck::typeck(&items, source, "<test>", &mut warnings).map_err(|_| "type error")?;
    let mut backend = CraneliftBackend::new().map_err(|e| format!("backend error: {e}"))?;
    backend
        .compile(&hir)
        .map_err(|e| format!("compile error: {e}"))?;
    backend.run().map_err(|e| format!("run error: {e}"))
}
