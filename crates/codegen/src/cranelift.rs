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

use crate::backend::CodegenBackend;

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
    decls: &'a [(String, cranelift_module::FuncId, Vec<HirParam>)],
    break_target: Option<ir::Block>,
    continue_target: Option<ir::Block>,
    vars: &'a mut HashMap<String, ir::Value>,
    pointer_type: ir::Type,
}

pub struct CraneliftBackend {
    module: JITModule,
    ctx: cranelift_codegen::Context,
    decls: Vec<(String, cranelift_module::FuncId, Vec<HirParam>)>,
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
            self.decls.push((f.name.clone(), func_id, f.params.clone()));
        }

        for (name, func_id, params) in &self.decls.clone() {
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
                let mut terminated = false;

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
                };
                for stmt in &func.body {
                    compile_stmt(
                        stmt,
                        &mut builder,
                        &mut terminated,
                        &mut ctx,
                    )?;
                }

                if !terminated {
                    builder.ins().return_(&[]);
                }

                builder.seal_all_blocks();
            }

            let ir_string = self.ctx.func.display().to_string();
            eprintln!("IR for {}:\n{}", name, ir_string);
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
        let Some(main_id) = self
            .decls
            .iter()
            .find(|(n, _, _)| n == "main")
            .map(|(_, id, _)| *id)
        else {
            return Ok(0);
        };

        let main_ptr = self.module.get_finalized_function(main_id);

        let main_fn: unsafe extern "C" fn() -> i64 = unsafe { std::mem::transmute(main_ptr) };
        let result = unsafe { main_fn() };
        Ok(result)
    }
}

fn compile_stmt(
    stmt: &HirStmt,
    builder: &mut FunctionBuilder,
    terminated: &mut bool,
    ctx: &mut CodegenCtx,
) -> Result<(), CraneliftError> {
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
            let val = compile_expr(value, builder, ctx)?;
            ctx.vars.insert(name.clone(), val);
            Ok(())
        }
        HirStmtKind::Expr(expr) => {
            compile_expr(expr, builder, ctx)?;
            Ok(())
        }
        HirStmtKind::Return(expr) => {
            match expr {
                Some(e) => {
                    let val = compile_expr(e, builder, ctx)?;
                    builder.ins().return_(&[val]);
                }
                None => {
                    builder.ins().return_(&[]);
                }
            }
            *terminated = true;
            Ok(())
        }
        HirStmtKind::Value(expr) => {
            let val = compile_expr(expr, builder, ctx)?;
            builder.ins().return_(&[val]);
            *terminated = true;
            Ok(())
        }
        HirStmtKind::Loop { body } => {
            let saved_break = ctx.break_target;
            let saved_continue = ctx.continue_target;

            let header = builder.create_block();
            let exit = builder.create_block();

            builder.ins().jump(header, &[]);

            builder.switch_to_block(header);

            ctx.break_target = Some(exit);
            ctx.continue_target = Some(header);

            let mut body_terminated = false;
            for stmt in body {
                if let HirStmt {
                    kind: HirStmtKind::Value(expr),
                    ..
                } = stmt
                {
                    compile_expr(expr, builder, ctx)?;
                } else {
                    compile_stmt(stmt, builder, &mut body_terminated, ctx)?;
                }
            }
            if !body_terminated {
                builder.ins().jump(header, &[]);
            }

            builder.seal_block(header);
            builder.switch_to_block(exit);
            builder.seal_block(exit);

            ctx.break_target = saved_break;
            ctx.continue_target = saved_continue;

            *terminated = false;
            Ok(())
        }
        HirStmtKind::Break => {
            match ctx.break_target {
                Some(target) => {
                    builder.ins().jump(target, &[]);
                }
                None => {
                    return Err(CraneliftError::Msg("break outside loop".to_string()));
                }
            }
            *terminated = true;
            Ok(())
        }
        HirStmtKind::Continue => {
            match ctx.continue_target {
                Some(target) => {
                    builder.ins().jump(target, &[]);
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

fn compile_expr(
    expr: &HirExpr,
    builder: &mut FunctionBuilder,
    ctx: &mut CodegenCtx,
) -> Result<ir::Value, CraneliftError> {
    match &expr.kind {
        HirExprKind::Int(v, _) => {
            let ty = ir_type_from_primitive(&expr.type_, ctx.pointer_type);
            Ok(builder.ins().iconst(ty, *v as i64))
        }
        HirExprKind::Float(v, _) => Ok(builder.ins().f64const(*v)),
        HirExprKind::Bool(b) => Ok(builder.ins().iconst(types::I8, *b as i64)),
        HirExprKind::Char(c) => Ok(builder.ins().iconst(types::I32, *c as i64)),
        HirExprKind::String(_) => Err(CraneliftError::Msg(
            "string expressions not supported in codegen yet".to_string(),
        )),
        HirExprKind::Ident(name) => ctx
            .vars
            .get(name)
            .copied()
            .ok_or_else(|| CraneliftError::Msg(format!("undefined variable `{name}`"))),
        HirExprKind::Binary { left, op, right } => {
            let left_val = compile_expr(left, builder, ctx)?;
            let right_val = compile_expr(right, builder, ctx)?;
            Ok(match op {
                BinaryOp::Add => builder.ins().iadd(left_val, right_val),
                BinaryOp::Sub => builder.ins().isub(left_val, right_val),
                BinaryOp::Mul => builder.ins().imul(left_val, right_val),
                BinaryOp::Div => builder.ins().sdiv(left_val, right_val),
                BinaryOp::Rem => builder.ins().srem(left_val, right_val),
                BinaryOp::Eq => builder.ins().icmp(IntCC::Equal, left_val, right_val),
                BinaryOp::Ne => builder.ins().icmp(IntCC::NotEqual, left_val, right_val),
                BinaryOp::Lt => builder
                    .ins()
                    .icmp(IntCC::SignedLessThan, left_val, right_val),
                BinaryOp::Gt => builder
                    .ins()
                    .icmp(IntCC::SignedGreaterThan, left_val, right_val),
                BinaryOp::Le => {
                    builder
                        .ins()
                        .icmp(IntCC::SignedLessThanOrEqual, left_val, right_val)
                }
                BinaryOp::Ge => {
                    builder
                        .ins()
                        .icmp(IntCC::SignedGreaterThanOrEqual, left_val, right_val)
                }
                BinaryOp::And => {
                    let zero = builder.ins().iconst(types::I8, 0);
                    let l = builder.ins().icmp(IntCC::NotEqual, left_val, zero);
                    let r = builder.ins().icmp(IntCC::NotEqual, right_val, zero);
                    builder.ins().band(l, r)
                }
                BinaryOp::Or => {
                    let zero = builder.ins().iconst(types::I8, 0);
                    let l = builder.ins().icmp(IntCC::NotEqual, left_val, zero);
                    let r = builder.ins().icmp(IntCC::NotEqual, right_val, zero);
                    builder.ins().bor(l, r)
                }
                BinaryOp::BitAnd => builder.ins().band(left_val, right_val),
                BinaryOp::BitOr => builder.ins().bor(left_val, right_val),
                BinaryOp::BitXor => builder.ins().bxor(left_val, right_val),
                BinaryOp::Shl => builder.ins().ishl(left_val, right_val),
                BinaryOp::Shr => builder.ins().sshr(left_val, right_val),
                BinaryOp::FloorDiv => {
                    let ty = builder.func.dfg.value_type(left_val);
                    let zero = builder.ins().iconst(ty, 0);
                    let one = builder.ins().iconst(ty, 1);
                    let q = builder.ins().sdiv(left_val, right_val);
                    let r = builder.ins().srem(left_val, right_val);
                    let r_ne_zero = builder.ins().icmp(IntCC::NotEqual, r, zero);
                    let sign_xor = builder.ins().bxor(left_val, right_val);
                    let signs_differ = builder.ins().icmp(IntCC::SignedLessThan, sign_xor, zero);
                    let adjust = builder.ins().band(r_ne_zero, signs_differ);
                    let q_minus_1 = builder.ins().isub(q, one);
                    builder.ins().select(adjust, q_minus_1, q)
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
                let callee_id = ctx
                    .decls
                    .iter()
                    .find(|(n, _, _)| n == name)
                    .map(|(_, id, _)| *id);
                if let Some(callee_id) = callee_id {
                    let mut call_args = Vec::new();
                    for arg in args {
                        let val = compile_expr(arg, builder, ctx)?;
                        call_args.push(val);
                    }
                    let sig = ctx.module.declare_func_in_func(callee_id, builder.func);
                    let inst = builder.ins().call(sig, &call_args);
                    let results = builder.inst_results(inst);
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
                compile_stmt(stmt, builder, &mut false, ctx)?;
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
            let elem_size = element_byte_size(element_type, ctx.pointer_type);
            let num_elements = elements.len() as u32;
            let slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                elem_size * num_elements,
                0,
            ));
            let base = builder.ins().stack_addr(ctx.pointer_type, slot, 0);
            for (i, element) in elements.iter().enumerate() {
                let val = compile_expr(element, builder, ctx)?;
                let offset = builder
                    .ins()
                    .iconst(ctx.pointer_type, (i as i64) * (elem_size as i64));
                let addr = builder.ins().iadd(base, offset);
                let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
                builder.ins().store(mflags, val, addr, 0);
            }
            Ok(base)
        }
        HirExprKind::Index { array, index } => {
            let array_ptr = compile_expr(array, builder, ctx)?;
            let index_val = compile_expr(index, builder, ctx)?;
            let index_ty = builder.func.dfg.value_type(index_val);
            let index_wide = if index_ty != ctx.pointer_type {
                builder.ins().uextend(ctx.pointer_type, index_val)
            } else {
                index_val
            };
            let elem_size = element_byte_size(&expr.type_, ctx.pointer_type);
            let size_val = builder.ins().iconst(ctx.pointer_type, elem_size as i64);
            let offset = builder.ins().imul(index_wide, size_val);
            let addr = builder.ins().iadd(array_ptr, offset);
            let result_ty = ir_type_from_primitive(&expr.type_, ctx.pointer_type);
            let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
            Ok(builder.ins().load(result_ty, mflags, addr, 0))
        }
        HirExprKind::If {
            condition,
            then_block,
            else_if,
            else_block,
        } => compile_expr_if(
            condition,
            then_block,
            else_if,
            else_block,
            expr,
            builder,
            ctx,
        ),
    }
}

fn compile_expr_if(
    condition: &HirExpr,
    then_block: &[HirStmt],
    else_if: &[(HirExpr, Vec<HirStmt>)],
    else_block: &Option<Vec<HirStmt>>,
    expr: &HirExpr,
    builder: &mut FunctionBuilder,
    ctx: &mut CodegenCtx,
) -> Result<ir::Value, CraneliftError> {
    let if_header = builder.create_block();
    let then_block_id = builder.create_block();
    let else_block_id = builder.create_block();
    let merge_block_id = builder.create_block();

    let non_unit_result = if !matches!(&expr.type_, Type::Primitive(Primitive::Unit)) {
        let result_type = ir_type_from_primitive(&expr.type_, ctx.pointer_type);
        let result_slot = builder.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            result_type.bytes(),
            0,
        ));
        let result_ptr = builder.ins().stack_addr(ctx.pointer_type, result_slot, 0);
        Some((result_type, result_ptr))
    } else {
        None
    };

    builder.ins().jump(if_header, &[]);
    builder.switch_to_block(if_header);

    let cond_val = compile_expr(condition, builder, ctx)?;
    builder
        .ins()
        .brif(cond_val, then_block_id, &[], else_block_id, &[]);
    builder.seal_block(if_header);

    compile_if_branch(
        then_block,
        then_block_id,
        merge_block_id,
        non_unit_result,
        builder,
        ctx,
    )?;
    compile_else_if_chain(
        else_if,
        else_block,
        else_block_id,
        merge_block_id,
        non_unit_result,
        builder,
        ctx,
    )?;

    builder.switch_to_block(merge_block_id);
    builder.seal_block(merge_block_id);

    let result_val = match non_unit_result {
        Some((result_type, result_ptr)) => {
            let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
            builder.ins().load(result_type, mflags, result_ptr, 0)
        }
        None => builder.ins().iconst(types::I8, 0),
    };

    let after = builder.create_block();
    builder.ins().jump(after, &[]);
    builder.switch_to_block(after);
    Ok(result_val)
}

fn compile_if_branch(
    stmts: &[HirStmt],
    block_id: ir::Block,
    merge_id: ir::Block,
    result: Option<(ir::Type, ir::Value)>,
    builder: &mut FunctionBuilder,
    ctx: &mut CodegenCtx,
) -> Result<(), CraneliftError> {
    builder.switch_to_block(block_id);
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
            let val = compile_expr(val_expr, builder, ctx)?;
            if let Some((_res_type, res_ptr)) = result {
                let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
                builder.ins().store(mflags, val, res_ptr, 0);
            }
            builder.ins().jump(merge_id, &[]);
            terminated = true;
            break;
        }
        compile_stmt(stmt, builder, &mut terminated, ctx)?;
        if !terminated && let Some(cur) = builder.current_block() {
            current = cur;
        }
    }
    if !terminated {
        builder.switch_to_block(current);
        builder.ins().jump(merge_id, &[]);
    }
    Ok(())
}

fn compile_else_if_chain(
    else_if: &[(HirExpr, Vec<HirStmt>)],
    else_block: &Option<Vec<HirStmt>>,
    else_block_id: ir::Block,
    merge_id: ir::Block,
    result: Option<(ir::Type, ir::Value)>,
    builder: &mut FunctionBuilder,
    ctx: &mut CodegenCtx,
) -> Result<(), CraneliftError> {
    builder.switch_to_block(else_block_id);
    for (cond, block) in else_if {
        let cond_val = compile_expr(cond, builder, ctx)?;
        let inner_then = builder.create_block();
        let inner_else = builder.create_block();
        builder
            .ins()
            .brif(cond_val, inner_then, &[], inner_else, &[]);
        compile_if_branch(
            block,
            inner_then,
            merge_id,
            result,
            builder,
            ctx,
        )?;
        builder.switch_to_block(inner_else);
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
                let val = compile_expr(val_expr, builder, ctx)?;
                if let Some((_res_type, res_ptr)) = result {
                    let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
                    builder.ins().store(mflags, val, res_ptr, 0);
                }
                builder.ins().jump(merge_id, &[]);
                terminated = true;
                break;
            }
            compile_stmt(stmt, builder, &mut terminated, ctx)?;
            if !terminated && let Some(cur) = builder.current_block() {
                current = cur;
            }
        }
        if !terminated {
            builder.switch_to_block(current);
            builder.ins().jump(merge_id, &[]);
        }
    } else {
        builder.ins().jump(merge_id, &[]);
    }
    Ok(())
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
