use std::collections::HashMap;
use std::fmt;

use cranelift_codegen::ir::{
    self, AbiParam, InstBuilder, Signature, StackSlotData, StackSlotKind, condcodes::IntCC, types,
};
use cranelift_codegen::isa::{self, CallConv};
use cranelift_codegen::settings;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};
use target_lexicon::Triple;

use vinyl_parser::ast::{BinaryOp, Primitive};
use vinyl_typecheck::hir::{
    HirExpr, HirExprKind, HirFunction, HirItem, HirItemKind, HirParam, HirStmt, HirStmtKind, Type,
};

use tracing::debug;

use crate::CodegenBackend;

#[derive(Debug)]
pub enum CraneliftError {
    Msg(String),
}

impl fmt::Display for CraneliftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CraneliftError::Msg(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for CraneliftError {}

struct CodegenCtx<'a> {
    module: &'a mut JITModule,
    decls: &'a [(String, cranelift_module::FuncId, Vec<HirParam>, Type)],
    break_target: Option<ir::Block>,
    continue_target: Option<ir::Block>,
    vars: &'a mut HashMap<String, ir::Value>,
    pointer_type: ir::Type,
    builder: &'a mut FunctionBuilder<'a>,
}

pub struct CraneliftBackend {
    module: JITModule,
    ctx: cranelift_codegen::Context,
    decls: Vec<(String, cranelift_module::FuncId, Vec<HirParam>, Type)>,
}

impl CraneliftBackend {
    pub fn new() -> Result<Self, CraneliftError> {
        let isa_builder = isa::lookup(Triple::host())
            .map_err(|e| CraneliftError::Msg(format!("isa lookup: {e}")))?;
        let flags = settings::Flags::new(settings::builder());
        let isa = isa_builder
            .finish(flags)
            .map_err(|e| CraneliftError::Msg(format!("isa finish: {e}")))?;
        let jit_builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        let module = JITModule::new(jit_builder);
        let ctx = module.make_context();
        Ok(CraneliftBackend {
            module,
            ctx,
            decls: Vec::new(),
        })
    }
}

impl CodegenBackend for CraneliftBackend {
    type Error = CraneliftError;

    fn compile(&mut self, items: &[HirItem]) -> Result<(), Self::Error> {
        let pointer_type = self.module.isa().pointer_type();
        for item in items {
            let HirItem {
                kind: HirItemKind::Function(f),
            } = item;
            let sig = hir_sig_to_clif(f, pointer_type);
            let func_id = self
                .module
                .declare_function(&f.name, Linkage::Export, &sig)
                .map_err(|e| CraneliftError::Msg(format!("declare {}: {e}", f.name)))?;
            self.decls.push((f.name.clone(), func_id, f.params.clone(), f.return_type.clone()));
        }

        for (name, func_id, params, _) in &self.decls.clone() {
            let func = items
                .iter()
                .find_map(|item| {
                    let HirItem {
                        kind: HirItemKind::Function(f),
                    } = item;
                    if &f.name == name { Some(f) } else { None }
                })
                .ok_or_else(|| CraneliftError::Msg(format!("function {name} not found")))?;

            self.ctx.clear();
            self.ctx.func.signature = hir_sig_to_clif(func, pointer_type);

            {
                let mut builder_ctx = FunctionBuilderContext::new();
                let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut builder_ctx);
                let entry = builder.create_block();
                builder.switch_to_block(entry);

                let mut vars = HashMap::new();

                for param in params.iter() {
                    let ty = param_type_to_clif(&param.type_, pointer_type);
                    let val = builder.append_block_param(entry, ty);
                    vars.insert(param.name.clone(), val);
                }

                let mut ctx = CodegenCtx {
                    module: &mut self.module,
                    decls: &self.decls,
                    break_target: None,
                    continue_target: None,
                    vars: &mut vars,
                    pointer_type,
                    builder: &mut builder,
                };

                let mut terminated = false;
                for stmt in &func.body {
                    ctx.compile_stmt(stmt, &mut terminated)?;
                }

                if !terminated {
                    ctx.builder.ins().return_(&[]);
                }

                ctx.builder.seal_all_blocks();
            }

            let ir_string = self.ctx.func.display().to_string();
            debug!("IR for {name}:\n{ir_string}");
            self.module
                .define_function(*func_id, &mut self.ctx)
                .map_err(|e| {
                    CraneliftError::Msg(format!("define {name}: {e}\nIR:\n{ir_string}"))
                })?;
            self.module.clear_context(&mut self.ctx);
        }

        self.module
            .finalize_definitions()
            .map_err(|e| CraneliftError::Msg(format!("finalize: {e}")))?;

        Ok(())
    }

    fn run(&self) -> Result<i64, Self::Error> {
        let Some((main_id, main_return)) = self
            .decls
            .iter()
            .find(|(n, _, _, _)| n == "main")
            .map(|(_, id, _, ret_type)| (id, ret_type))
        else {
            return Ok(0);
        };

        if matches!(main_return, Type::Primitive(Primitive::Unit)) {
            let main_fn: unsafe extern "C" fn() = unsafe { std::mem::transmute(self.module.get_finalized_function(*main_id)) };
            unsafe { main_fn() };
            return Ok(0);
        }

        let main_ptr = self.module.get_finalized_function(*main_id);
        let main_fn: unsafe extern "C" fn() -> i64 = unsafe { std::mem::transmute(main_ptr) };
        let result = unsafe { main_fn() };
        Ok(result)
    }
}

impl<'a> CodegenCtx<'a> {
    fn compile_stmt(&mut self, stmt: &HirStmt, terminated: &mut bool) -> Result<(), CraneliftError> {
        if *terminated {
            return Ok(());
        }

        match &stmt.kind {
            HirStmtKind::Let {
                name,
                type_: _,
                value,
                ..
            } => {
                let val = self.compile_expr(value)?;
                self.vars.insert(name.clone(), val);
                Ok(())
            }
            HirStmtKind::Expr(expr) => {
                self.compile_expr(expr)?;
                Ok(())
            }
            HirStmtKind::Return(expr) => {
                match expr {
                    Some(e) => {
                        let val = self.compile_expr(e)?;
                        self.builder.ins().return_(&[val]);
                    }
                    None => {
                        self.builder.ins().return_(&[]);
                    }
                }
                *terminated = true;
                Ok(())
            }
            HirStmtKind::Value(expr) => {
                let val = self.compile_expr(expr)?;
                if matches!(expr.type_, Type::Primitive(Primitive::Unit)) {
                    self.builder.ins().return_(&[]);
                } else {
                    self.builder.ins().return_(&[val]);
                }
                *terminated = true;
                Ok(())
            }
            HirStmtKind::Loop { body } => {
                let saved_break = self.break_target;
                let saved_continue = self.continue_target;

                let header = self.builder.create_block();
                let exit = self.builder.create_block();

                self.builder.ins().jump(header, &[]);
                self.builder.switch_to_block(header);

                self.break_target = Some(exit);
                self.continue_target = Some(header);

                let mut body_terminated = false;
                for stmt in body {
                    if let HirStmt {
                        kind: HirStmtKind::Value(expr),
                        ..
                    } = stmt
                    {
                        self.compile_expr(expr)?;
                    } else {
                        self.compile_stmt(stmt, &mut body_terminated)?;
                    }
                }
                if !body_terminated {
                    self.builder.ins().jump(header, &[]);
                }

                self.builder.seal_block(header);
                self.builder.switch_to_block(exit);
                self.builder.seal_block(exit);

                self.break_target = saved_break;
                self.continue_target = saved_continue;

                *terminated = false;
                Ok(())
            }
            HirStmtKind::Break => {
                match self.break_target {
                    Some(target) => {
                        self.builder.ins().jump(target, &[]);
                    }
                    None => {
                        return Err(CraneliftError::Msg("break outside loop".to_string()));
                    }
                }
                *terminated = true;
                Ok(())
            }
            HirStmtKind::Continue => {
                match self.continue_target {
                    Some(target) => {
                        self.builder.ins().jump(target, &[]);
                    }
                    None => {
                        return Err(CraneliftError::Msg("continue outside loop".to_string()));
                    }
                }
                *terminated = true;
                Ok(())
            }
        }
    }

    fn compile_expr(&mut self, expr: &HirExpr) -> Result<ir::Value, CraneliftError> {
        match &expr.kind {
            HirExprKind::Int(v, _) => {
                let ty = ir_type_from_primitive(&expr.type_, self.pointer_type);
                Ok(self.builder.ins().iconst(ty, *v as i64))
            }
            HirExprKind::Float(v, _) => Ok(self.builder.ins().f64const(*v)),
            HirExprKind::Unit => Ok(self.builder.ins().iconst(types::I8, 0)),
            HirExprKind::Bool(b) => Ok(self.builder.ins().iconst(types::I8, *b as i64)),
            HirExprKind::Char(c) => Ok(self.builder.ins().iconst(types::I32, *c as i64)),
            HirExprKind::String(_) => Err(CraneliftError::Msg(
                "string expressions not supported in codegen yet".to_string(),
            )),
            HirExprKind::Ident(name) => self
                .vars
                .get(name)
                .copied()
                .ok_or_else(|| CraneliftError::Msg(format!("undefined variable `{name}`"))),
            HirExprKind::Binary { left, op, right } => {
                let left_val = self.compile_expr(left)?;
                let right_val = self.compile_expr(right)?;
                Ok(match op {
                    BinaryOp::Add => self.builder.ins().iadd(left_val, right_val),
                    BinaryOp::Sub => self.builder.ins().isub(left_val, right_val),
                    BinaryOp::Mul => self.builder.ins().imul(left_val, right_val),
                    BinaryOp::Div => self.builder.ins().sdiv(left_val, right_val),
                    BinaryOp::Rem => self.builder.ins().srem(left_val, right_val),
                    BinaryOp::Eq => self.builder.ins().icmp(IntCC::Equal, left_val, right_val),
                    BinaryOp::Ne => self.builder.ins().icmp(IntCC::NotEqual, left_val, right_val),
                    BinaryOp::Lt => self
                        .builder
                        .ins()
                        .icmp(IntCC::SignedLessThan, left_val, right_val),
                    BinaryOp::Gt => self
                        .builder
                        .ins()
                        .icmp(IntCC::SignedGreaterThan, left_val, right_val),
                    BinaryOp::Le => {
                        self.builder
                            .ins()
                            .icmp(IntCC::SignedLessThanOrEqual, left_val, right_val)
                    }
                    BinaryOp::Ge => {
                        self.builder
                            .ins()
                            .icmp(IntCC::SignedGreaterThanOrEqual, left_val, right_val)
                    }
                    BinaryOp::And => {
                        let zero = self.builder.ins().iconst(types::I8, 0);
                        let l = self.builder.ins().icmp(IntCC::NotEqual, left_val, zero);
                        let r = self.builder.ins().icmp(IntCC::NotEqual, right_val, zero);
                        self.builder.ins().band(l, r)
                    }
                    BinaryOp::Or => {
                        let zero = self.builder.ins().iconst(types::I8, 0);
                        let l = self.builder.ins().icmp(IntCC::NotEqual, left_val, zero);
                        let r = self.builder.ins().icmp(IntCC::NotEqual, right_val, zero);
                        self.builder.ins().bor(l, r)
                    }
                    BinaryOp::BitAnd => self.builder.ins().band(left_val, right_val),
                    BinaryOp::BitOr => self.builder.ins().bor(left_val, right_val),
                    BinaryOp::BitXor => self.builder.ins().bxor(left_val, right_val),
                    BinaryOp::Shl => self.builder.ins().ishl(left_val, right_val),
                    BinaryOp::Shr => self.builder.ins().sshr(left_val, right_val),
                    BinaryOp::FloorDiv => {
                        let ty = self.builder.func.dfg.value_type(left_val);
                        let zero = self.builder.ins().iconst(ty, 0);
                        let one = self.builder.ins().iconst(ty, 1);
                        let q = self.builder.ins().sdiv(left_val, right_val);
                        let r = self.builder.ins().srem(left_val, right_val);
                        let r_ne_zero = self.builder.ins().icmp(IntCC::NotEqual, r, zero);
                        let sign_xor = self.builder.ins().bxor(left_val, right_val);
                        let signs_differ = self
                            .builder
                            .ins()
                            .icmp(IntCC::SignedLessThan, sign_xor, zero);
                        let adjust = self.builder.ins().band(r_ne_zero, signs_differ);
                        let q_minus_1 = self.builder.ins().isub(q, one);
                        self.builder.ins().select(adjust, q_minus_1, q)
                    }
                    BinaryOp::Pow => {
                        return Err(CraneliftError::Msg(
                            "power operator not supported in codegen yet".to_string(),
                        ));
                    }
                    BinaryOp::Range | BinaryOp::RangeInclusive => {
                        return Err(CraneliftError::Msg(
                            "range operators not supported in codegen".to_string(),
                        ));
                    }
                })
            }
            HirExprKind::Call { function, args } => {
                if let HirExprKind::Ident(name) = &function.kind {
                    let callee_id = self
                        .decls
                        .iter()
                        .find(|(n, _, _, _)| n == name)
                        .map(|(_, id, _, _)| *id);
                    if let Some(callee_id) = callee_id {
                        let mut call_args = Vec::new();
                        for arg in args {
                            let val = self.compile_expr(arg)?;
                            call_args.push(val);
                        }
                        let sig = self
                            .module
                            .declare_func_in_func(callee_id, self.builder.func);
                        let inst = self.builder.ins().call(sig, &call_args);
                        let results = self.builder.inst_results(inst);
                        if results.is_empty() {
                            Err(CraneliftError::Msg(
                                "void function call used as expression".to_string(),
                            ))
                        } else {
                            Ok(results[0])
                        }
                    } else {
                        Err(CraneliftError::Msg(format!("undefined function `{name}`")))
                    }
                } else {
                    Err(CraneliftError::Msg(
                        "call target must be a function name".to_string(),
                    ))
                }
            }
            HirExprKind::Block(stmts) => {
                for stmt in stmts {
                    self.compile_stmt(stmt, &mut false)?;
                }
                Err(CraneliftError::Msg(
                    "blocks as expressions not supported in codegen".to_string(),
                ))
            }
            HirExprKind::Array(elements) => {
                let element_type = match &expr.type_ {
                    Type::Array { element, .. } => element.as_ref(),
                    _ => &Type::Primitive(Primitive::Int32),
                };
                let elem_size = element_byte_size(element_type, self.pointer_type);
                let num_elements = elements.len() as u32;
                let slot = self.builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    elem_size * num_elements,
                    0,
                ));
                let base = self.builder.ins().stack_addr(self.pointer_type, slot, 0);
                for (i, element) in elements.iter().enumerate() {
                    let val = self.compile_expr(element)?;
                    let offset = self
                        .builder
                        .ins()
                        .iconst(self.pointer_type, (i as i64) * (elem_size as i64));
                    let addr = self.builder.ins().iadd(base, offset);
                    let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
                    self.builder.ins().store(mflags, val, addr, 0);
                }
                Ok(base)
            }
            HirExprKind::Index { array, index } => {
                let array_ptr = self.compile_expr(array)?;
                let index_val = self.compile_expr(index)?;
                let index_ty = self.builder.func.dfg.value_type(index_val);
                let index_wide = if index_ty != self.pointer_type {
                    self.builder.ins().uextend(self.pointer_type, index_val)
                } else {
                    index_val
                };
                let elem_size = element_byte_size(&expr.type_, self.pointer_type);
                let size_val = self.builder.ins().iconst(self.pointer_type, elem_size as i64);
                let offset = self.builder.ins().imul(index_wide, size_val);
                let addr = self.builder.ins().iadd(array_ptr, offset);
                let result_ty = ir_type_from_primitive(&expr.type_, self.pointer_type);
                let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
                Ok(self.builder.ins().load(result_ty, mflags, addr, 0))
            }
            HirExprKind::If {
                condition,
                then_block,
                else_if,
                else_block,
            } => {
                let if_expr = IfExprBundle {
                    condition,
                    then_block,
                    else_if,
                    else_block,
                    result_type: &expr.type_,
                };
                self.compile_expr_if(if_expr)
            }
        }
    }

    fn compile_expr_if(&mut self, if_expr: IfExprBundle) -> Result<ir::Value, CraneliftError> {
        let IfExprBundle { condition, then_block, else_if, else_block, result_type } = if_expr;

        let if_header = self.builder.create_block();
        let then_block_id = self.builder.create_block();
        let else_block_id = self.builder.create_block();
        let merge_block_id = self.builder.create_block();

        let result_slot = if !matches!(result_type, Type::Primitive(Primitive::Unit)) {
            let result_type = ir_type_from_primitive(result_type, self.pointer_type);
            let slot = self.builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                result_type.bytes(),
                0,
            ));
            let result_ptr = self.builder.ins().stack_addr(self.pointer_type, slot, 0);
            Some((result_type, result_ptr))
        } else {
            None
        };

        self.builder.ins().jump(if_header, &[]);
        self.builder.switch_to_block(if_header);

        let cond_val = self.compile_expr(condition)?;
        self.builder
            .ins()
            .brif(cond_val, then_block_id, &[], else_block_id, &[]);
        self.builder.seal_block(if_header);

        let mut if_ctx = IfBranchCtx {
            merge_block: merge_block_id,
            result_slot,
        };

        self.compile_if_branch(then_block, then_block_id, &mut if_ctx)?;
        self.compile_else_if_chain(else_if, else_block, else_block_id, &mut if_ctx)?;

        self.builder.switch_to_block(merge_block_id);
        self.builder.seal_block(merge_block_id);

        let result_val = match if_ctx.result_slot {
            Some((result_type, result_ptr)) => {
                let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
                self.builder.ins().load(result_type, mflags, result_ptr, 0)
            }
            None => self.builder.ins().iconst(types::I8, 0),
        };

        let after = self.builder.create_block();
        self.builder.ins().jump(after, &[]);
        self.builder.switch_to_block(after);
        Ok(result_val)
    }

    fn compile_if_branch(
        &mut self,
        stmts: &[HirStmt],
        block_id: ir::Block,
        if_ctx: &mut IfBranchCtx,
    ) -> Result<(), CraneliftError> {
        self.builder.switch_to_block(block_id);
        let mut terminated = false;
        let mut current = block_id;
        for (i, stmt) in stmts.iter().enumerate() {
            let is_last = i == stmts.len() - 1;
            if is_last
                && let HirStmt {
                    kind: HirStmtKind::Value(val_expr),
                    ..
                } = stmt
            {
                let val = self.compile_expr(val_expr)?;
                if let Some((_res_type, res_ptr)) = if_ctx.result_slot {
                    let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
                    self.builder.ins().store(mflags, val, res_ptr, 0);
                }
                self.builder.ins().jump(if_ctx.merge_block, &[]);
                terminated = true;
                break;
            }
            self.compile_stmt(stmt, &mut terminated)?;
            if !terminated && let Some(cur) = self.builder.current_block() {
                current = cur;
            }
        }
        if !terminated {
            self.builder.switch_to_block(current);
            self.builder.ins().jump(if_ctx.merge_block, &[]);
        }
        Ok(())
    }

    fn compile_else_if_chain(
        &mut self,
        else_if: &[(HirExpr, Vec<HirStmt>)],
        else_block: &Option<Vec<HirStmt>>,
        else_block_id: ir::Block,
        if_ctx: &mut IfBranchCtx,
    ) -> Result<(), CraneliftError> {
        self.builder.switch_to_block(else_block_id);
        for (cond, block) in else_if {
            let cond_val = self.compile_expr(cond)?;
            let inner_then = self.builder.create_block();
            let inner_else = self.builder.create_block();
            self.builder
                .ins()
                .brif(cond_val, inner_then, &[], inner_else, &[]);
            self.compile_if_branch(block, inner_then, if_ctx)?;
            self.builder.switch_to_block(inner_else);
        }
        if let Some(stmts) = else_block {
            let mut terminated = false;
            let mut current = else_block_id;
            for (i, stmt) in stmts.iter().enumerate() {
                let is_last = i == stmts.len() - 1;
                if is_last
                    && let HirStmt {
                        kind: HirStmtKind::Value(val_expr),
                        ..
                    } = stmt
                {
                    let val = self.compile_expr(val_expr)?;
                    if let Some((_res_type, res_ptr)) = if_ctx.result_slot {
                        let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
                        self.builder.ins().store(mflags, val, res_ptr, 0);
                    }
                    self.builder.ins().jump(if_ctx.merge_block, &[]);
                    terminated = true;
                    break;
                }
                self.compile_stmt(stmt, &mut terminated)?;
                if !terminated && let Some(cur) = self.builder.current_block() {
                    current = cur;
                }
            }
            if !terminated {
                self.builder.switch_to_block(current);
                self.builder.ins().jump(if_ctx.merge_block, &[]);
            }
        } else {
            self.builder.ins().jump(if_ctx.merge_block, &[]);
        }
        Ok(())
    }
}

struct IfExprBundle<'a> {
    condition: &'a HirExpr,
    then_block: &'a [HirStmt],
    else_if: &'a [(HirExpr, Vec<HirStmt>)],
    else_block: &'a Option<Vec<HirStmt>>,
    result_type: &'a Type,
}

struct IfBranchCtx {
    merge_block: ir::Block,
    result_slot: Option<(ir::Type, ir::Value)>,
}

fn element_byte_size(t: &Type, pointer_type: ir::Type) -> u32 {
    let ptr_size = pointer_type.bytes();
    match t {
        Type::Primitive(p) => match p {
            Primitive::Int8 | Primitive::UInt8 | Primitive::Bool => 1,
            Primitive::Int16 | Primitive::UInt16 => 2,
            Primitive::Int32 | Primitive::UInt32 | Primitive::Float32 | Primitive::Char => 4,
            Primitive::Int64 | Primitive::UInt64 | Primitive::Float64 => 8,
            Primitive::Int128 | Primitive::UInt128 => 16,
            Primitive::ISize | Primitive::USize | Primitive::String => ptr_size,
            Primitive::Unit => 0,
        },
        Type::Ref(_) => ptr_size,
        Type::Array { element, size } => element_byte_size(element, pointer_type) * (*size as u32),
        _ => ptr_size,
    }
}

fn param_type_to_clif(t: &Type, pointer_type: ir::Type) -> types::Type {
    match t {
        Type::Primitive(Primitive::Int32) => types::I32,
        Type::Primitive(Primitive::Int64) => types::I64,
        Type::Primitive(Primitive::ISize) => pointer_type,
        Type::Primitive(Primitive::USize) => pointer_type,
        Type::Primitive(Primitive::Float64) => types::F64,
        Type::Primitive(Primitive::Bool) => types::I8,
        Type::Primitive(Primitive::Char) => types::I32,
        _ => types::I64,
    }
}

fn ir_type_from_primitive(t: &Type, pointer_type: ir::Type) -> ir::Type {
    match t {
        Type::Primitive(Primitive::Int32) => types::I32,
        Type::Primitive(Primitive::Int64) => types::I64,
        Type::Primitive(Primitive::ISize) => pointer_type,
        Type::Primitive(Primitive::USize) => pointer_type,
        Type::Primitive(Primitive::Float64) => types::F64,
        Type::Primitive(Primitive::Bool) => types::I8,
        Type::Primitive(Primitive::Char) => types::I32,
        _ => types::I64,
    }
}

fn hir_sig_to_clif(func: &HirFunction, pointer_type: ir::Type) -> Signature {
    #[cfg(windows)]
    let call_conv = CallConv::WindowsFastcall;
    #[cfg(not(windows))]
    let call_conv = CallConv::SystemV;

    let mut sig = Signature::new(call_conv);

    for param in &func.params {
        sig.params.push(AbiParam::new(param_type_to_clif(
            &param.type_,
            pointer_type,
        )));
    }

    match &func.return_type {
        Type::Primitive(Primitive::Unit) => {}
        other => {
            sig.returns
                .push(AbiParam::new(param_type_to_clif(other, pointer_type)));
        }
    }

    sig
}
