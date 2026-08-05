use std::collections::{HashMap, HashSet};

use cranelift_codegen::ir::{self};
use cranelift_frontend::{FunctionBuilder, Variable};
use cranelift_jit::JITModule;
use cranelift_module::FuncId;
use vinyl_typecheck::hir::{HirItemKind, HirParam, Type};

pub struct VarInfo {
    pub slot: VarSlot,
    pub vinyl_type: Type,
}

#[derive(Clone, Copy)]
pub enum VarSlot {
    Value(ir::Value),
    Variable(Variable),
    StackSlot(ir::StackSlot, ir::Type),
}

pub struct ModuleEnv<'a> {
    pub module: &'a mut JITModule,
    pub decls: &'a [(String, FuncId, Vec<HirParam>, Type)],
    pub print_func: FuncId,
    pub types: &'a HashMap<String, HirItemKind>,
    pub pointer_type: ir::Type,
}

pub struct FuncEnv<'a> {
    pub builder: &'a mut FunctionBuilder<'a>,
    pub vars: &'a mut HashMap<String, VarInfo>,
    pub ref_vars: &'a HashSet<String>,
    pub break_target: Option<ir::Block>,
    pub continue_target: Option<ir::Block>,
    pub return_type: Type,
    pub sret_ptr: Option<ir::Value>,
}

pub struct CodegenCtx<'a> {
    pub module: ModuleEnv<'a>,
    pub func: FuncEnv<'a>,
}
