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
}

struct IfBranches<'a> {
    condition: &'a HirExpr,
    then_block: &'a [HirStmt],
    else_if: &'a [(HirExpr, Vec<HirStmt>)],
    else_block: &'a Option<Vec<HirStmt>>,
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
                };
                for stmt in &func.body {
                    compile_stmt(
                        stmt,
                        &mut builder,
                        &mut vars,
                        &mut terminated,
                        &mut ctx,
                        pointer_type,
                    )?;
                }

                if !terminated {
                    builder.ins().return_(&[]);
                }

                builder.seal_all_blocks();
            }

            self.module
                .define_function(*func_id, &mut self.ctx)
                .map_err(|e| CraneliftError::Msg(format!("define {name}: {e}")))?;
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
    vars: &mut HashMap<String, ir::Value>,
    terminated: &mut bool,
    ctx: &mut CodegenCtx,
    pointer_type: ir::Type,
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
            let val = compile_expr(value, builder, vars, ctx, pointer_type)?;
            vars.insert(name.clone(), val);
            Ok(())
        }
        HirStmtKind::Expr(expr) => {
            compile_expr(expr, builder, vars, ctx, pointer_type)?;
            Ok(())
        }
        HirStmtKind::Return(expr) => {
            match expr {
                Some(e) => {
                    let val = compile_expr(e, builder, vars, ctx, pointer_type)?;
                    builder.ins().return_(&[val]);
                }
                None => {
                    builder.ins().return_(&[]);
                }
            }
            *terminated = true;
            Ok(())
        }
        HirStmtKind::If {
            condition,
            then_block,
            else_if,
            else_block,
        } => {
            let branches = IfBranches {
                condition,
                then_block,
                else_if,
                else_block,
            };
            compile_if(branches, builder, vars, terminated, ctx, pointer_type)
        }
    }
}

fn compile_if(
    branches: IfBranches,
    builder: &mut FunctionBuilder,
    vars: &mut HashMap<String, ir::Value>,
    terminated: &mut bool,
    ctx: &mut CodegenCtx,
    pointer_type: ir::Type,
) -> Result<(), CraneliftError> {
    let cond_val = compile_expr(branches.condition, builder, vars, ctx, pointer_type)?;

    let then_block_id = builder.create_block();
    let else_block_id = builder.create_block();
    let merge_block_id = builder.create_block();

    builder
        .ins()
        .brif(cond_val, then_block_id, &[], else_block_id, &[]);

    builder.switch_to_block(then_block_id);
    let mut then_terminated = false;
    for stmt in branches.then_block {
        compile_stmt(stmt, builder, vars, &mut then_terminated, ctx, pointer_type)?;
    }
    if !then_terminated {
        builder.ins().jump(merge_block_id, &[]);
    }

    builder.switch_to_block(else_block_id);

    for (cond, block) in branches.else_if {
        let else_cond = compile_expr(cond, builder, vars, ctx, pointer_type)?;
        let inner_then = builder.create_block();
        let inner_else = builder.create_block();

        builder
            .ins()
            .brif(else_cond, inner_then, &[], inner_else, &[]);

        builder.switch_to_block(inner_then);
        let mut inner_terminated = false;
        for stmt in block {
            compile_stmt(
                stmt,
                builder,
                vars,
                &mut inner_terminated,
                ctx,
                pointer_type,
            )?;
        }
        if !inner_terminated {
            builder.ins().jump(merge_block_id, &[]);
        }

        builder.switch_to_block(inner_else);
    }

    let mut else_terminated = false;
    if let Some(else_stmts) = branches.else_block {
        for stmt in else_stmts {
            compile_stmt(stmt, builder, vars, &mut else_terminated, ctx, pointer_type)?;
        }
    }
    if !else_terminated {
        builder.ins().jump(merge_block_id, &[]);
    }

    builder.switch_to_block(merge_block_id);
    builder.seal_block(merge_block_id);

    *terminated = then_terminated && else_terminated;

    Ok(())
}

fn compile_expr(
    expr: &HirExpr,
    builder: &mut FunctionBuilder,
    vars: &mut HashMap<String, ir::Value>,
    ctx: &mut CodegenCtx,
    pointer_type: ir::Type,
) -> Result<ir::Value, CraneliftError> {
    match &expr.kind {
        HirExprKind::Int(v, _) => {
            let ty = ir_type_from_primitive(&expr.type_, pointer_type);
            Ok(builder.ins().iconst(ty, *v as i64))
        }
        HirExprKind::Float(v, _) => Ok(builder.ins().f64const(*v)),
        HirExprKind::Bool(b) => Ok(builder.ins().iconst(types::I8, *b as i64)),
        HirExprKind::Char(c) => Ok(builder.ins().iconst(types::I32, *c as i64)),
        HirExprKind::String(_) => Err(CraneliftError::Msg(
            "string expressions not supported in codegen yet".to_string(),
        )),
        HirExprKind::Ident(name) => vars
            .get(name)
            .copied()
            .ok_or_else(|| CraneliftError::Msg(format!("undefined variable `{name}`"))),
        HirExprKind::Binary { left, op, right } => {
            let left_val = compile_expr(left, builder, vars, ctx, pointer_type)?;
            let right_val = compile_expr(right, builder, vars, ctx, pointer_type)?;
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
                        let val = compile_expr(arg, builder, vars, ctx, pointer_type)?;
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
                compile_stmt(stmt, builder, vars, &mut false, ctx, pointer_type)?;
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
            let elem_size = element_byte_size(element_type, pointer_type);
            let num_elements = elements.len() as u32;
            let slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                elem_size * num_elements,
                0,
            ));
            let base = builder.ins().stack_addr(pointer_type, slot, 0);
            for (i, element) in elements.iter().enumerate() {
                let val = compile_expr(element, builder, vars, ctx, pointer_type)?;
                let offset = builder
                    .ins()
                    .iconst(pointer_type, (i as i64) * (elem_size as i64));
                let addr = builder.ins().iadd(base, offset);
                let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
                builder.ins().store(mflags, val, addr, 0);
            }
            Ok(base)
        }
        HirExprKind::Index { array, index } => {
            let array_ptr = compile_expr(array, builder, vars, ctx, pointer_type)?;
            let index_val = compile_expr(index, builder, vars, ctx, pointer_type)?;
            let index_ty = builder.func.dfg.value_type(index_val);
            let index_wide = if index_ty != pointer_type {
                builder.ins().uextend(pointer_type, index_val)
            } else {
                index_val
            };
            let elem_size = element_byte_size(&expr.type_, pointer_type);
            let size_val = builder.ins().iconst(pointer_type, elem_size as i64);
            let offset = builder.ins().imul(index_wide, size_val);
            let addr = builder.ins().iadd(array_ptr, offset);
            let result_ty = ir_type_from_primitive(&expr.type_, pointer_type);
            let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
            Ok(builder.ins().load(result_ty, mflags, addr, 0))
        }
    }
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
    let mut sig = Signature::new(CallConv::SystemV);

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
