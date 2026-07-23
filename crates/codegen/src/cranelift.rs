use std::collections::{HashMap, HashSet};

use crate::layout;
use std::fmt;

use cranelift_codegen::ir::{
    self, AbiParam, InstBuilder, Signature, StackSlotData, StackSlotKind, condcodes::IntCC, types,
};
use cranelift_codegen::isa::{self, CallConv};
use cranelift_codegen::settings;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};
use target_lexicon::Triple;

use vinyl_parser::ast::operator::{BinaryOp, UnaryOp};
use vinyl_parser::ast::types::Primitive;
use vinyl_typecheck::hir::{
    AssignOp, HirAssignTarget, HirEnumVariantData, HirExpr, HirExprKind, HirFunction, HirItem,
    HirItemKind, HirParam, HirStatement, HirStatementKind, Type,
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

struct VarInfo {
    slot: VarSlot,
    vinyl_type: Type,
}

#[derive(Clone, Copy)]
enum VarSlot {
    Value(ir::Value),
    Variable(Variable),
    StackSlot(ir::StackSlot, ir::Type),
}

struct CodegenCtx<'a> {
    module: &'a mut JITModule,
    decls: &'a [(String, cranelift_module::FuncId, Vec<HirParam>, Type)],
    types: &'a HashMap<String, HirItemKind>,
    break_target: Option<ir::Block>,
    continue_target: Option<ir::Block>,
    vars: &'a mut HashMap<String, VarInfo>,
    ref_vars: &'a HashSet<String>,
    pointer_type: ir::Type,
    builder: &'a mut FunctionBuilder<'a>,
}

pub struct CraneliftBackend {
    module: JITModule,
    ctx: cranelift_codegen::Context,
    decls: Vec<(String, cranelift_module::FuncId, Vec<HirParam>, Type)>,
    types: HashMap<String, HirItemKind>,
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
            types: HashMap::new(),
        })
    }
}

impl CodegenBackend for CraneliftBackend {
    type Error = CraneliftError;

    fn compile(&mut self, items: &[HirItem]) -> Result<(), Self::Error> {
        let pointer_type = self.module.isa().pointer_type();

        for item in items {
            let name = match &item.kind {
                HirItemKind::Struct(s) => Some(s.name.clone()),
                HirItemKind::TupleStruct(t) => Some(t.name.clone()),
                HirItemKind::Enum(e) => Some(e.name.clone()),
                _ => None,
            };
            if let Some(name) = name {
                self.types.insert(name, item.kind.clone());
            }
        }

        for item in items {
            let HirItemKind::Function(f) = &item.kind else {
                continue;
            };
            let sig = hir_sig_to_clif(f, pointer_type);
            let func_id = self
                .module
                .declare_function(&f.name, Linkage::Export, &sig)
                .map_err(|e| CraneliftError::Msg(format!("declare {}: {e}", f.name)))?;
            self.decls.push((
                f.name.clone(),
                func_id,
                f.params.clone(),
                f.return_type.clone(),
            ));
        }

        for (name, func_id, params, _) in &self.decls.clone() {
            let func = items
                .iter()
                .find_map(|item| {
                    if let HirItemKind::Function(f) = &item.kind {
                        if &f.name == name { Some(f) } else { None }
                    } else {
                        None
                    }
                })
                .ok_or_else(|| CraneliftError::Msg(format!("function {name} not found")))?;

            self.ctx.clear();
            self.ctx.func.signature = hir_sig_to_clif(func, pointer_type);

            {
                let mut builder_ctx = FunctionBuilderContext::new();
                let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut builder_ctx);
                let entry = builder.create_block();
                builder.switch_to_block(entry);

                let ref_vars = prescan_function_body(&func.body);
                let mut vars = HashMap::new();

                for param in params.iter() {
                    let ty = param_type_to_clif(&param.type_, pointer_type);
                    let val = builder.append_block_param(entry, ty);
                    let mode = var_mode(&param.name, param.mutable, &ref_vars);
                    let (slot, _) =
                        build_var_info(&mut builder, &param.type_, ty, val, mode, pointer_type);
                    vars.insert(
                        param.name.clone(),
                        VarInfo {
                            slot,
                            vinyl_type: param.type_.clone(),
                        },
                    );
                }

                let mut ctx = CodegenCtx {
                    module: &mut self.module,
                    decls: &self.decls,
                    types: &self.types,
                    break_target: None,
                    continue_target: None,
                    vars: &mut vars,
                    ref_vars: &ref_vars,
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
            let main_fn: unsafe extern "C" fn() =
                unsafe { std::mem::transmute(self.module.get_finalized_function(*main_id)) };
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
    fn read_var(&mut self, name: &str) -> Result<ir::Value, CraneliftError> {
        let val = self.read_var_raw(name)?;
        let info = self
            .vars
            .get(name)
            .ok_or_else(|| CraneliftError::Msg(format!("undefined variable `{name}`")))?;
        if let Type::Ref(inner) = &info.vinyl_type {
            let inner_ty = ir_type_from_primitive(inner.as_ref(), self.pointer_type);
            let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
            Ok(self.builder.ins().load(inner_ty, mflags, val, 0))
        } else {
            Ok(val)
        }
    }

    fn read_var_raw(&mut self, name: &str) -> Result<ir::Value, CraneliftError> {
        let info = self
            .vars
            .get(name)
            .ok_or_else(|| CraneliftError::Msg(format!("undefined variable `{name}`")))?;
        match info.slot {
            VarSlot::Value(v) => Ok(v),
            VarSlot::Variable(v) => Ok(self.builder.use_var(v)),
            VarSlot::StackSlot(slot, ty) => {
                let addr = self.builder.ins().stack_addr(self.pointer_type, slot, 0);
                let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
                let val = self.builder.ins().load(ty, mflags, addr, 0);
                Ok(val)
            }
        }
    }

    fn write_var(&mut self, name: &str, val: ir::Value) -> Result<(), CraneliftError> {
        let info = self
            .vars
            .get_mut(name)
            .ok_or_else(|| CraneliftError::Msg(format!("undefined variable `{name}`")))?;
        match info.slot {
            VarSlot::Value(_) => Err(CraneliftError::Msg(format!(
                "cannot write to immutable variable `{name}`"
            ))),
            VarSlot::Variable(v) => {
                self.builder.def_var(v, val);
                Ok(())
            }
            VarSlot::StackSlot(slot, _ty) => {
                let addr = self.builder.ins().stack_addr(self.pointer_type, slot, 0);
                let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
                self.builder.ins().store(mflags, val, addr, 0);
                Ok(())
            }
        }
    }

    fn compile_stmt(
        &mut self,
        stmt: &HirStatement,
        terminated: &mut bool,
    ) -> Result<(), CraneliftError> {
        if *terminated {
            return Ok(());
        }

        match &stmt.kind {
            HirStatementKind::Let {
                name,
                type_,
                value,
                mutable,
            } => {
                let clif_type = ir_type_from_primitive(type_, self.pointer_type);
                let val = self.compile_expr(value)?;
                let mode = var_mode(name, *mutable, self.ref_vars);
                let (slot, _) =
                    build_var_info(self.builder, type_, clif_type, val, mode, self.pointer_type);
                self.vars.insert(
                    name.clone(),
                    VarInfo {
                        slot,
                        vinyl_type: type_.clone(),
                    },
                );
                Ok(())
            }
            HirStatementKind::Expr(expr) => {
                self.compile_expr(expr)?;
                Ok(())
            }
            HirStatementKind::Return(expr) => {
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
            HirStatementKind::Value(expr) => {
                let val = self.compile_expr(expr)?;
                if matches!(expr.type_, Type::Primitive(Primitive::Unit)) {
                    self.builder.ins().return_(&[]);
                } else {
                    self.builder.ins().return_(&[val]);
                }
                *terminated = true;
                Ok(())
            }
            HirStatementKind::Loop { body } => {
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
                    if let HirStatement {
                        kind: HirStatementKind::Value(expr),
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
            HirStatementKind::Break => {
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
            HirStatementKind::Continue => {
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
            HirStatementKind::Assign { target, op, value } => {
                let val = self.compile_expr(value)?;
                self.compile_assign_target(target, op, val)?;
                Ok(())
            }
        }
    }

    fn compile_assign_target(
        &mut self,
        target: &HirAssignTarget,
        op: &AssignOp,
        value: ir::Value,
    ) -> Result<(), CraneliftError> {
        let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
        let is_compound = *op != AssignOp::Eq;

        let write_val = if is_compound {
            // Read current value, apply compound operator, produce final value
            let current = match target {
                HirAssignTarget::Ident(name) => self.read_var(name)?,
                HirAssignTarget::Deref(inner) => {
                    let ptr = match &inner.kind {
                        HirExprKind::Ident(name) => self.read_var_raw(name)?,
                        _ => self.compile_expr(inner)?,
                    };
                    let ty = self.builder.func.dfg.value_type(value);
                    self.builder.ins().load(ty, mflags, ptr, 0)
                }
                HirAssignTarget::Index { array, index } => {
                    let array_ptr = self.compile_expr(array)?;
                    let index_val = self.compile_expr(index)?;
                    let addr = self.compute_index_addr(array_ptr, index_val, target)?;
                    let ty = self.builder.func.dfg.value_type(value);
                    self.builder.ins().load(ty, mflags, addr, 0)
                }
                HirAssignTarget::Field { .. } => {
                    return Err(CraneliftError::Msg(
                        "compound assignment to struct field not supported".to_string(),
                    ));
                }
            };
            self.apply_compound_op(current, value, op)
        } else {
            value
        };

        match target {
            HirAssignTarget::Ident(name) => self.write_var(name, write_val),
            HirAssignTarget::Deref(inner) => {
                let ptr = match &inner.kind {
                    HirExprKind::Ident(name) => self.read_var_raw(name)?,
                    _ => self.compile_expr(inner)?,
                };
                self.builder.ins().store(mflags, write_val, ptr, 0);
                Ok(())
            }
            HirAssignTarget::Index { array, index } => {
                let array_ptr = self.compile_expr(array)?;
                let index_val = self.compile_expr(index)?;
                let addr = self.compute_index_addr(array_ptr, index_val, target)?;
                self.builder.ins().store(mflags, write_val, addr, 0);
                Ok(())
            }
            HirAssignTarget::Field { object, name: _ } => {
                let _ = self.compile_expr(object)?;
                Err(CraneliftError::Msg(
                    "struct field assignment not supported".to_string(),
                ))
            }
        }
    }

    fn compute_index_addr(
        &mut self,
        array_ptr: ir::Value,
        index_val: ir::Value,
        target: &HirAssignTarget,
    ) -> Result<ir::Value, CraneliftError> {
        let index_ty = self.builder.func.dfg.value_type(index_val);
        let index_wide = if index_ty != self.pointer_type {
            self.builder.ins().uextend(self.pointer_type, index_val)
        } else {
            index_val
        };
        let elem_type = match extract_array_element_type(target) {
            Some(t) => t,
            None => &Type::Primitive(Primitive::Int32),
        };
        let elem_size = element_byte_size(elem_type, self.pointer_type);
        let size_val = self
            .builder
            .ins()
            .iconst(self.pointer_type, elem_size as i64);
        let offset = self.builder.ins().imul(index_wide, size_val);
        Ok(self.builder.ins().iadd(array_ptr, offset))
    }

    fn apply_compound_op(
        &mut self,
        current: ir::Value,
        value: ir::Value,
        op: &AssignOp,
    ) -> ir::Value {
        match op {
            AssignOp::Eq => value,
            AssignOp::AddEq => self.builder.ins().iadd(current, value),
            AssignOp::SubEq => self.builder.ins().isub(current, value),
            AssignOp::MulEq => self.builder.ins().imul(current, value),
            AssignOp::DivEq => self.builder.ins().sdiv(current, value),
            AssignOp::RemEq => self.builder.ins().srem(current, value),
            AssignOp::BitAndEq => self.builder.ins().band(current, value),
            AssignOp::BitOrEq => self.builder.ins().bor(current, value),
            AssignOp::BitXorEq => self.builder.ins().bxor(current, value),
            AssignOp::ShlEq => self.builder.ins().ishl(current, value),
            AssignOp::ShrEq => self.builder.ins().sshr(current, value),
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
            HirExprKind::Ident(name) => self.read_var(name),
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
                    BinaryOp::Ne => self
                        .builder
                        .ins()
                        .icmp(IntCC::NotEqual, left_val, right_val),
                    BinaryOp::Lt => {
                        self.builder
                            .ins()
                            .icmp(IntCC::SignedLessThan, left_val, right_val)
                    }
                    BinaryOp::Gt => {
                        self.builder
                            .ins()
                            .icmp(IntCC::SignedGreaterThan, left_val, right_val)
                    }
                    BinaryOp::Le => {
                        self.builder
                            .ins()
                            .icmp(IntCC::SignedLessThanOrEqual, left_val, right_val)
                    }
                    BinaryOp::Ge => self.builder.ins().icmp(
                        IntCC::SignedGreaterThanOrEqual,
                        left_val,
                        right_val,
                    ),
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
                        let signs_differ =
                            self.builder
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
            HirExprKind::Unary { op, operand } => {
                let val = self.compile_expr(operand)?;
                Ok(match op {
                    UnaryOp::Neg => {
                        let ty = self.builder.func.dfg.value_type(val);
                        let zero = self.builder.ins().iconst(ty, 0);
                        self.builder.ins().isub(zero, val)
                    }
                    UnaryOp::Not => {
                        let ty = self.builder.func.dfg.value_type(val);
                        let one = self.builder.ins().iconst(ty, 1);
                        self.builder.ins().bxor(val, one)
                    }
                    UnaryOp::Ref => self.compound_ref_expr(operand),
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
                            Ok(self.builder.ins().iconst(types::I32, 0))
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
            HirExprKind::Index { array, index, .. } => {
                let array_ptr = self.compile_expr(array)?;
                let index_val = self.compile_expr(index)?;
                let index_ty = self.builder.func.dfg.value_type(index_val);
                let index_wide = if index_ty != self.pointer_type {
                    self.builder.ins().uextend(self.pointer_type, index_val)
                } else {
                    index_val
                };
                let elem_size = element_byte_size(&expr.type_, self.pointer_type);
                let size_val = self
                    .builder
                    .ins()
                    .iconst(self.pointer_type, elem_size as i64);
                let offset = self.builder.ins().imul(index_wide, size_val);
                let addr = self.builder.ins().iadd(array_ptr, offset);
                let result_ty = ir_type_from_primitive(&expr.type_, self.pointer_type);
                let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
                Ok(self.builder.ins().load(result_ty, mflags, addr, 0))
            }
            HirExprKind::Ref(inner) => {
                match inner.as_ref() {
                    HirExpr {
                        kind: HirExprKind::Ident(name),
                        ..
                    } => {
                        // &x: return the stack address of x
                        match self.vars.get(name) {
                            Some(VarInfo {
                                slot: VarSlot::StackSlot(slot, _),
                                ..
                            }) => Ok(self.builder.ins().stack_addr(self.pointer_type, *slot, 0)),
                            _ => Err(CraneliftError::Msg(format!(
                                "cannot take reference of variable `{name}`: not stored in a stack slot"
                            ))),
                        }
                    }
                    _ => Err(CraneliftError::Msg(
                        "reference operator only supports identifiers".to_string(),
                    )),
                }
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
            HirExprKind::Tuple(elements, _) => {
                let ptr_size = self.pointer_type.bytes();
                let tuple_type = &expr.type_;
                let total_size = layout::size_of(tuple_type, ptr_size);
                let slot = self.builder.create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    total_size,
                    0,
                ));
                let base = self.builder.ins().stack_addr(self.pointer_type, slot, 0);
                if let Type::Tuple(element_types) = tuple_type {
                    for (i, element) in elements.iter().enumerate() {
                        let val = self.compile_expr(element)?;
                        let offset = layout::tuple_field_offset(i, element_types, ptr_size);
                        let addr = if offset == 0 {
                            base
                        } else {
                            let off_val =
                                self.builder.ins().iconst(self.pointer_type, offset as i64);
                            self.builder.ins().iadd(base, off_val)
                        };
                        let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
                        self.builder.ins().store(mflags, val, addr, 0);
                    }
                }
                Ok(base)
            }
            HirExprKind::EnumVariant {
                type_name,
                variant_index,
                payload,
            } => self.compile_enum_variant(type_name, *variant_index, payload, &expr.type_),
            HirExprKind::FieldAccess { object, name, .. } => {
                let ptr_size = self.pointer_type.bytes();
                let obj = self.compile_expr(object)?;
                let offset = match &object.type_ {
                    Type::Tuple(element_types) => {
                        let index: usize = name.parse().map_err(|_| {
                            CraneliftError::Msg(format!("invalid tuple index `{name}`"))
                        })?;
                        layout::tuple_field_offset(index, element_types, ptr_size)
                    }
                    Type::Named(type_name) => match self.types.get(type_name) {
                        Some(HirItemKind::Struct(s)) => {
                            let field_types: Vec<(String, Type)> = s
                                .fields
                                .iter()
                                .map(|f| (f.name.clone(), f.type_.clone()))
                                .collect();
                            let (_, field_layouts) =
                                layout::struct_layout(&field_types, s.repr_c, ptr_size);
                            let field_idx = s
                                .fields
                                .iter()
                                .position(|f| f.name == *name)
                                .ok_or_else(|| {
                                    CraneliftError::Msg(format!(
                                        "struct `{type_name}` has no field `{name}`"
                                    ))
                                })?;
                            field_layouts[field_idx].1.offset
                        }
                        Some(HirItemKind::TupleStruct(t)) => {
                            let index: usize = name.parse().map_err(|_| {
                                CraneliftError::Msg(format!("invalid tuple struct field `{name}`"))
                            })?;
                            layout::tuple_field_offset(index, &t.types, ptr_size)
                        }
                        _ => {
                            return Err(CraneliftError::Msg(format!(
                                "cannot access field on type `{type_name}`"
                            )));
                        }
                    },
                    _ => {
                        return Err(CraneliftError::Msg(format!(
                            "field access not supported for type {:?}",
                            object.type_
                        )));
                    }
                };
                let field_clif = ir_type_from_primitive(&expr.type_, self.pointer_type);
                let addr = if offset == 0 {
                    obj
                } else {
                    let off_val = self.builder.ins().iconst(self.pointer_type, offset as i64);
                    self.builder.ins().iadd(obj, off_val)
                };
                let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
                Ok(self.builder.ins().load(field_clif, mflags, addr, 0))
            }
        }
    }

    fn compound_ref_expr(&mut self, _operand: &HirExpr) -> ir::Value {
        unreachable!("UnaryOp::Ref not used directly; HirExprKind::Ref handles it")
    }

    fn compile_expr_if(&mut self, if_expr: IfExprBundle) -> Result<ir::Value, CraneliftError> {
        let IfExprBundle {
            condition,
            then_block,
            else_if,
            else_block,
            result_type,
        } = if_expr;

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
        stmts: &[HirStatement],
        block_id: ir::Block,
        if_ctx: &mut IfBranchCtx,
    ) -> Result<(), CraneliftError> {
        self.builder.switch_to_block(block_id);
        let mut terminated = false;
        for (i, stmt) in stmts.iter().enumerate() {
            if terminated {
                break;
            }
            if i == stmts.len() - 1
                && let HirStatement {
                    kind: HirStatementKind::Value(val_expr),
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
        }
        if !terminated {
            self.builder.ins().jump(if_ctx.merge_block, &[]);
        }
        Ok(())
    }

    fn compile_else_if_chain(
        &mut self,
        else_if: &[(HirExpr, Vec<HirStatement>)],
        else_block: &Option<Vec<HirStatement>>,
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
            for (i, stmt) in stmts.iter().enumerate() {
                let is_last = i == stmts.len() - 1;
                if is_last
                    && let HirStatement {
                        kind: HirStatementKind::Value(val_expr),
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
            }
            if !terminated {
                self.builder.ins().jump(if_ctx.merge_block, &[]);
            }
        } else {
            self.builder.ins().jump(if_ctx.merge_block, &[]);
        }
        Ok(())
    }

    fn compile_enum_variant(
        &mut self,
        type_name: &str,
        variant_index: usize,
        payload: &[HirExpr],
        _result_type: &Type,
    ) -> Result<ir::Value, CraneliftError> {
        let ptr_size = self.pointer_type.bytes();
        let hir_enum = match self.types.get(type_name) {
            Some(HirItemKind::Enum(e)) => e,
            _ => {
                return Err(CraneliftError::Msg(format!(
                    "type `{type_name}` is not an enum"
                )));
            }
        };
        let variant = &hir_enum.variants[variant_index];
        let payload_types: Vec<Type> = match &variant.data {
            Some(HirEnumVariantData::Tuple(types)) => types.clone(),
            Some(HirEnumVariantData::Struct(fields)) => {
                fields.iter().map(|f| f.type_.clone()).collect()
            }
            None => Vec::new(),
        };
        let (total_size, data_offset, _disc_size) =
            layout::enum_layout(std::slice::from_ref(&payload_types), ptr_size);
        if total_size > 8 {
            return Err(CraneliftError::Msg(format!(
                "enum `{type_name}` is too large (> 8 bytes) for codegen yet"
            )));
        }
        let slot = self.builder.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            8,
            0,
        ));
        let base = self.builder.ins().stack_addr(self.pointer_type, slot, 0);
        let zero = self.builder.ins().iconst(types::I64, 0);
        let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
        self.builder.ins().store(mflags, zero, base, 0);
        let disc_val = self.builder.ins().iconst(types::I8, variant_index as i64);
        self.builder.ins().store(mflags, disc_val, base, 0);
        let mut data_offset_acc = 0u32;
        for (i, elem) in payload.iter().enumerate() {
            let val = self.compile_expr(elem)?;
            let elem_align = layout::align_of(&payload_types[i], ptr_size);
            data_offset_acc = layout::align_up(data_offset_acc, elem_align);
            let field_addr = if data_offset + data_offset_acc == 0 {
                base
            } else {
                let off = self
                    .builder
                    .ins()
                    .iconst(self.pointer_type, (data_offset + data_offset_acc) as i64);
                self.builder.ins().iadd(base, off)
            };
            self.builder.ins().store(mflags, val, field_addr, 0);
            data_offset_acc += layout::size_of(&payload_types[i], ptr_size);
        }
        Ok(self.builder.ins().load(types::I64, mflags, base, 0))
    }
}

struct IfExprBundle<'a> {
    condition: &'a HirExpr,
    then_block: &'a [HirStatement],
    else_if: &'a [(HirExpr, Vec<HirStatement>)],
    else_block: &'a Option<Vec<HirStatement>>,
    result_type: &'a Type,
}

struct IfBranchCtx {
    merge_block: ir::Block,
    result_slot: Option<(ir::Type, ir::Value)>,
}

enum VarMode {
    Value,
    Variable,
    StackSlot,
}

fn var_mode(name: &str, mutable: bool, ref_vars: &HashSet<String>) -> VarMode {
    if ref_vars.contains(name) {
        VarMode::StackSlot
    } else if mutable {
        VarMode::Variable
    } else {
        VarMode::Value
    }
}

fn build_var_info(
    builder: &mut FunctionBuilder,
    _vtype: &Type,
    clif_type: ir::Type,
    initial_val: ir::Value,
    mode: VarMode,
    pointer_type: ir::Type,
) -> (VarSlot, ir::Type) {
    match mode {
        VarMode::Value => (VarSlot::Value(initial_val), clif_type),
        VarMode::Variable => {
            let var = builder.declare_var(clif_type);
            builder.def_var(var, initial_val);
            (VarSlot::Variable(var), clif_type)
        }
        VarMode::StackSlot => {
            let ptr_size = pointer_type.bytes();
            let slot = builder.create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                ptr_size.max(clif_type.bytes()),
                0,
            ));
            let addr = builder.ins().stack_addr(pointer_type, slot, 0);
            let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
            builder.ins().store(mflags, initial_val, addr, 0);
            (VarSlot::StackSlot(slot, clif_type), clif_type)
        }
    }
}

fn prescan_function_body(body: &[HirStatement]) -> HashSet<String> {
    let mut refed = HashSet::new();
    prescan_stmts(body, &mut refed);
    refed
}

fn prescan_stmts(stmts: &[HirStatement], refed: &mut HashSet<String>) {
    for stmt in stmts {
        match &stmt.kind {
            HirStatementKind::Let { value, .. } => prescan_expr(value, refed),
            HirStatementKind::Expr(e) | HirStatementKind::Value(e) => prescan_expr(e, refed),
            HirStatementKind::Return(Some(e)) => prescan_expr(e, refed),
            HirStatementKind::Return(None) => {}
            HirStatementKind::Loop { body } => prescan_stmts(body, refed),
            HirStatementKind::Break | HirStatementKind::Continue => {}
            HirStatementKind::Assign { target, value, .. } => {
                if let HirAssignTarget::Deref(e) = target {
                    prescan_expr(e, refed);
                }
                prescan_expr(value, refed);
            }
        }
    }
}

fn prescan_expr(expr: &HirExpr, refed: &mut HashSet<String>) {
    match &expr.kind {
        HirExprKind::Ident(_name) => {}
        HirExprKind::Ref(inner) => {
            if let HirExpr {
                kind: HirExprKind::Ident(name),
                ..
            } = inner.as_ref()
            {
                refed.insert(name.clone());
            }
            prescan_expr(inner, refed);
        }
        HirExprKind::Unary { operand, .. } => prescan_expr(operand, refed),
        HirExprKind::Binary { left, right, .. } => {
            prescan_expr(left, refed);
            prescan_expr(right, refed);
        }
        HirExprKind::Call { function, args } => {
            prescan_expr(function, refed);
            for arg in args {
                prescan_expr(arg, refed);
            }
        }
        HirExprKind::Block(stmts) => prescan_stmts(stmts, refed),
        HirExprKind::Array(elements) => {
            for elem in elements {
                prescan_expr(elem, refed);
            }
        }
        HirExprKind::Index { array, index, .. } => {
            prescan_expr(array, refed);
            prescan_expr(index, refed);
        }
        HirExprKind::If {
            condition,
            then_block,
            else_if,
            else_block,
        } => {
            prescan_expr(condition, refed);
            prescan_stmts(then_block, refed);
            for (cond, block) in else_if {
                prescan_expr(cond, refed);
                prescan_stmts(block, refed);
            }
            if let Some(block) = else_block {
                prescan_stmts(block, refed);
            }
        }
        HirExprKind::Tuple(elements, _) => {
            for elem in elements {
                prescan_expr(elem, refed);
            }
        }
        HirExprKind::EnumVariant { payload, .. } => {
            for elem in payload {
                prescan_expr(elem, refed);
            }
        }
        HirExprKind::FieldAccess { object, .. } => {
            prescan_expr(object, refed);
        }
        HirExprKind::Int(..)
        | HirExprKind::Float(..)
        | HirExprKind::String(..)
        | HirExprKind::Bool(..)
        | HirExprKind::Unit
        | HirExprKind::Char(..) => {}
    }
}

fn extract_array_element_type(target: &HirAssignTarget) -> Option<&Type> {
    match target {
        HirAssignTarget::Index { array, .. } => {
            if let Type::Array { element, .. } = &array.type_ {
                Some(element.as_ref())
            } else {
                None
            }
        }
        _ => None,
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
        Type::Primitive(Primitive::ISize) | Type::Ref(_) => pointer_type,
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
        Type::Primitive(Primitive::ISize) | Type::Ref(_) => pointer_type,
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
