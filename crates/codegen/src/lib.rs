use vinyl_typecheck::hir::HirItem;

pub trait CodegenBackend {
    type Error: std::error::Error;
    fn compile(&mut self, items: &[HirItem]) -> Result<(), Self::Error>;
    fn run(&self) -> Result<i64, Self::Error>;
}

pub mod cranelift;

pub use cranelift::CraneliftBackend;
