use std::collections::{HashMap, HashSet};

use cranelift_codegen::ir::{self};
use cranelift_frontend::{FunctionBuilder, Variable};
use cranelift_jit::JITModule;
use cranelift_module::FuncId;
use vinyl_typecheck::hir::{HirItemKind, HirParam, Type};

/// A local variable and its Vinyl type.
pub struct VarInfo {
    pub slot: VarSlot,
    pub vinyl_type: Type,
}

/// Cranelift representation of a local variable.
#[derive(Clone, Copy)]
pub enum VarSlot {
    Value(ir::Value),
    Variable(Variable),
    StackSlot(ir::StackSlot, ir::Type),
}

/// State shared by all functions in one compiled module.
pub struct ModuleEnv<'a> {
    pub module: &'a mut JITModule,
    pub decls: &'a [(String, FuncId, Vec<HirParam>, Type)],
    pub print_func: FuncId,
    pub types: &'a HashMap<String, HirItemKind>,
    pub intrinsics: &'a HashSet<String>,
    pub pointer_type: ir::Type,
}

/// State associated with one function body.
pub struct FuncEnv<'a> {
    pub builder: &'a mut FunctionBuilder<'a>,
    pub vars: &'a mut HashMap<String, VarInfo>,
    pub ref_vars: &'a HashSet<String>,
    pub break_target: Option<ir::Block>,
    pub continue_target: Option<ir::Block>,
    pub return_type: Type,
    pub sret_ptr: Option<ir::Value>,
}

/// Backend context used while emitting one function.
pub struct CodegenCtx<'a> {
    pub module: ModuleEnv<'a>,
    pub func: FuncEnv<'a>,
}
