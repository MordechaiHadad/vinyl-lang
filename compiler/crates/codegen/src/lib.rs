use vinyl_typecheck::hir::HirItem;

/// Backend-independent entry point for compiling and executing a Vinyl module.
pub trait CodegenBackend {
    /// Backend-specific diagnostic error.
    type Error: miette::Diagnostic + Send + Sync + 'static;

    /// Compiles the supplied typechecked items.
    fn compile(&mut self, items: &[HirItem]) -> Result<(), Self::Error>;

    /// Executes the compiled `main` function and returns its integer result.
    fn run(&self) -> Result<i64, Self::Error>;
}

/// Backend-neutral target and ABI policy.
pub mod abi;
pub mod cranelift;
pub mod error;
pub mod layout;
pub mod locals;
pub mod runtime;

pub use cranelift::CraneliftBackend;
pub use error::CraneliftError;
