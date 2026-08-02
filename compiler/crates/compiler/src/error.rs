use miette::Diagnostic;
use thiserror::Error;
use vinyl_parser::ParserDiagnostic;
use vinyl_resolver::ResolveDiagnostic;
use vinyl_typecheck::TypeDiagnostic;

#[derive(Debug, Error, Diagnostic)]
pub enum CompileError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    Parse(#[from] ParserDiagnostic),
    #[error(transparent)]
    #[diagnostic(transparent)]
    TypeDiagnostic(#[from] TypeDiagnostic),
    #[error("io error: {0}")]
    #[diagnostic(code(compiler::io_error))]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    #[diagnostic(code(compiler::module_error))]
    Module(#[from] ModuleError),
    #[error("module resolution error: {0}")]
    #[diagnostic(code(compiler::module_resolution_error))]
    ModResolve(#[from] ResolveDiagnostic),
}

#[derive(Debug, Error, Diagnostic)]
#[error("{message}")]
#[diagnostic(
    help("check the module path and file structure"),
    code(compiler::module_error)
)]
pub struct ModuleError {
    pub message: String,
}
