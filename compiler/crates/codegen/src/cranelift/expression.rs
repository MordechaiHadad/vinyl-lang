use cranelift_codegen::ir::{
    self, InstBuilder, StackSlotData, StackSlotKind, condcodes::{FloatCC, IntCC}, types,
};
use cranelift_module::Module;

use vinyl_parser::ast::operator::{BinaryOp, UnaryOp};
use vinyl_parser::ast::types::Primitive;
use vinyl_typecheck::hir::{
    HirEnumVariantData, HirExpression, HirExpressionKind, HirItemKind, HirStatement,
    HirStatementKind, Type,
};

use super::state::CodegenCtx;
use super::types::{element_byte_size, ir_type_from_primitive, is_large_aggregate};
use crate::CraneliftError;

pub struct IfExprBundle<'a> {
    pub condition: &'a HirExpression,
    pub then_block: &'a [HirStatement],
    pub else_if: &'a [(HirExpression, Vec<HirStatement>)],
    pub else_block: &'a Option<Vec<HirStatement>>,
    pub result_type: &'a Type,
}

pub struct IfBranchCtx {
    pub merge_block: ir::Block,
    pub result_slot: Option<(ir::Type, ir::Value)>,
}

impl<'a> CodegenCtx<'a> {
    pub fn compile_expr(&mut self, expr: &HirExpression) -> Result<ir::Value, CraneliftError> {
        match &expr.kind {
            HirExpressionKind::Int(v, _) => {
                let ty = ir_type_from_primitive(&expr.type_, self.module.pointer_type);
                Ok(self.func.builder.ins().iconst(ty, *v as i64))
            }
            HirExpressionKind::Float(v, _) => Ok(self.func.builder.ins().f64const(*v)),
            HirExpressionKind::Unit(_) => Ok(self.func.builder.ins().iconst(types::I8, 0)),
            HirExpressionKind::Bool(b, _) => {
                Ok(self.func.builder.ins().iconst(types::I8, *b as i64))
            }
            HirExpressionKind::Char(c, _) => {
                Ok(self.func.builder.ins().iconst(types::I32, *c as i64))
            }
            HirExpressionKind::String(..) => Err(CraneliftError::Msg(
                "string expressions not supported in codegen yet".to_string(),
            )),
            HirExpressionKind::Ident(name, _) => self.read_var(name),
            HirExpressionKind::Binary {
                left, op, right, ..
            } => {
                let left_val = self.compile_expr(left)?;
                let right_val = self.compile_expr(right)?;
                Ok(match op {
                    BinaryOp::Add => self.func.builder.ins().iadd(left_val, right_val),
                    BinaryOp::Sub => self.func.builder.ins().isub(left_val, right_val),
                    BinaryOp::Mul => self.func.builder.ins().imul(left_val, right_val),
                    BinaryOp::Div => self.func.builder.ins().sdiv(left_val, right_val),
                    BinaryOp::Rem => self.func.builder.ins().srem(left_val, right_val),
                    BinaryOp::Eq => self
                        .func
                        .builder
                        .ins()
                        .icmp(IntCC::Equal, left_val, right_val),
                    BinaryOp::Ne => {
                        self.func
                            .builder
                            .ins()
                            .icmp(IntCC::NotEqual, left_val, right_val)
                    }
                    BinaryOp::Lt => {
                        self.func
                            .builder
                            .ins()
                            .icmp(IntCC::SignedLessThan, left_val, right_val)
                    }
                    BinaryOp::Gt => {
                        self.func
                            .builder
                            .ins()
                            .icmp(IntCC::SignedGreaterThan, left_val, right_val)
                    }
                    BinaryOp::Le => self.func.builder.ins().icmp(
                        IntCC::SignedLessThanOrEqual,
                        left_val,
                        right_val,
                    ),
                    BinaryOp::Ge => self.func.builder.ins().icmp(
                        IntCC::SignedGreaterThanOrEqual,
                        left_val,
                        right_val,
                    ),
                    BinaryOp::And => {
                        let zero = self.func.builder.ins().iconst(types::I8, 0);
                        let l = self
                            .func
                            .builder
                            .ins()
                            .icmp(IntCC::NotEqual, left_val, zero);
                        let r = self
                            .func
                            .builder
                            .ins()
                            .icmp(IntCC::NotEqual, right_val, zero);
                        self.func.builder.ins().band(l, r)
                    }
                    BinaryOp::Or => {
                        let zero = self.func.builder.ins().iconst(types::I8, 0);
                        let l = self
                            .func
                            .builder
                            .ins()
                            .icmp(IntCC::NotEqual, left_val, zero);
                        let r = self
                            .func
                            .builder
                            .ins()
                            .icmp(IntCC::NotEqual, right_val, zero);
                        self.func.builder.ins().bor(l, r)
                    }
                    BinaryOp::BitAnd => self.func.builder.ins().band(left_val, right_val),
                    BinaryOp::BitOr => self.func.builder.ins().bor(left_val, right_val),
                    BinaryOp::BitXor => self.func.builder.ins().bxor(left_val, right_val),
                    BinaryOp::Shl => self.func.builder.ins().ishl(left_val, right_val),
                    BinaryOp::Shr => self.func.builder.ins().sshr(left_val, right_val),
                    BinaryOp::FloorDiv => {
                        let ty = self.func.builder.func.dfg.value_type(left_val);
                        let zero = self.func.builder.ins().iconst(ty, 0);
                        let one = self.func.builder.ins().iconst(ty, 1);
                        let q = self.func.builder.ins().sdiv(left_val, right_val);
                        let r = self.func.builder.ins().srem(left_val, right_val);
                        let r_ne_zero = self.func.builder.ins().icmp(IntCC::NotEqual, r, zero);
                        let sign_xor = self.func.builder.ins().bxor(left_val, right_val);
                        let signs_differ =
                            self.func
                                .builder
                                .ins()
                                .icmp(IntCC::SignedLessThan, sign_xor, zero);
                        let adjust = self.func.builder.ins().band(r_ne_zero, signs_differ);
                        let q_minus_1 = self.func.builder.ins().isub(q, one);
                        self.func.builder.ins().select(adjust, q_minus_1, q)
                    }
                    BinaryOp::Pow => self.compile_pow(left_val, right_val, right)?,
                    BinaryOp::Range | BinaryOp::RangeInclusive => {
                        return Err(CraneliftError::Msg(
                            "range operators not supported in codegen".to_string(),
                        ));
                    }
                })
            }
            HirExpressionKind::Unary { op, operand, .. } => {
                let val = self.compile_expr(operand)?;
                Ok(match op {
                    UnaryOp::Neg => {
                        let ty = self.func.builder.func.dfg.value_type(val);
                        let zero = self.func.builder.ins().iconst(ty, 0);
                        self.func.builder.ins().isub(zero, val)
                    }
                    UnaryOp::Not => {
                        let ty = self.func.builder.func.dfg.value_type(val);
                        let one = self.func.builder.ins().iconst(ty, 1);
                        self.func.builder.ins().bxor(val, one)
                    }
                    UnaryOp::Ref => {
                        unreachable!("UnaryOp::Ref not used directly; HirExprKind::Ref handles it")
                    }
                })
            }
            HirExpressionKind::Call { function, args, .. } => {
                if let HirExpressionKind::Ident(name, _) = &function.kind {
                    let callee_info = self
                        .module
                        .decls
                        .iter()
                        .find(|(n, _, _, _)| n == name)
                        .map(|(n, id, params, ret_type)| (n, id, params, ret_type));
                    if let Some((_, callee_id, callee_params, callee_ret_type)) = callee_info {
                        let ptr_size = self.module.pointer_type.bytes();
                        let mut call_args = Vec::new();
                        let ret_size =
                            crate::layout::size_of(callee_ret_type, self.module.types, ptr_size);
                        let needs_sret =
                            !matches!(callee_ret_type, Type::Primitive(Primitive::Unit))
                                && ret_size > 8;
                        // SRet: allocate return slot, push its address first
                        // todo: baseline sret, multi-register return replaces this
                        let sret_slot = if needs_sret {
                            let slot =
                                self.func
                                    .builder
                                    .create_sized_stack_slot(StackSlotData::new(
                                        StackSlotKind::ExplicitSlot,
                                        ret_size,
                                        0,
                                    ));
                            let addr = self.func.builder.ins().stack_addr(
                                self.module.pointer_type,
                                slot,
                                0,
                            );
                            call_args.push(addr);
                            Some((slot, addr))
                        } else {
                            None
                        };
                        for (i, arg) in args.iter().enumerate() {
                            let param_type = &callee_params[i].type_;
                            let param_size =
                                crate::layout::size_of(param_type, self.module.types, ptr_size);
                            if param_size > 8 {
                                let val = self.compile_expr(arg)?;
                                // todo: baseline by-ref, multi-register decomposition replaces this
                                call_args.push(val);
                            } else {
                                let val = self.compile_expr(arg)?;
                                call_args.push(val);
                            }
                        }
                        let sig = self
                            .module
                            .module
                            .declare_func_in_func(*callee_id, self.func.builder.func);
                        let inst = self.func.builder.ins().call(sig, &call_args);
                        let results = self.func.builder.inst_results(inst);
                        if let Some((_slot, addr)) = sret_slot {
                            Ok(addr)
                        } else if results.is_empty() {
                            Ok(self.func.builder.ins().iconst(types::I32, 0))
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
            HirExpressionKind::Block(stmts, _) => {
                for stmt in stmts {
                    self.compile_stmt(stmt, &mut false)?;
                }
                Err(CraneliftError::Msg(
                    "blocks as expressions not supported in codegen".to_string(),
                ))
            }
            HirExpressionKind::Array(elements, _) => {
                let element_type = match &expr.type_ {
                    Type::Array { element, .. } => element.as_ref(),
                    _ => &Type::Primitive(Primitive::Int32),
                };
                let elem_size = element_byte_size(element_type, self.module.pointer_type);
                let num_elements = elements.len() as u32;
                let slot = self
                    .func
                    .builder
                    .create_sized_stack_slot(StackSlotData::new(
                        StackSlotKind::ExplicitSlot,
                        elem_size * num_elements,
                        0,
                    ));
                let base = self
                    .func
                    .builder
                    .ins()
                    .stack_addr(self.module.pointer_type, slot, 0);
                for (i, element) in elements.iter().enumerate() {
                    let val = self.compile_expr(element)?;
                    let offset = self
                        .func
                        .builder
                        .ins()
                        .iconst(self.module.pointer_type, (i as i64) * (elem_size as i64));
                    let addr = self.func.builder.ins().iadd(base, offset);
                    let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
                    self.func.builder.ins().store(mflags, val, addr, 0);
                }
                Ok(base)
            }
            HirExpressionKind::Index { array, index, .. } => {
                let array_ptr = self.compile_expr(array)?;
                let index_val = self.compile_expr(index)?;
                let index_ty = self.func.builder.func.dfg.value_type(index_val);
                let index_wide = if index_ty != self.module.pointer_type {
                    self.func
                        .builder
                        .ins()
                        .uextend(self.module.pointer_type, index_val)
                } else {
                    index_val
                };
                let elem_size = element_byte_size(&expr.type_, self.module.pointer_type);
                let size_val = self
                    .func
                    .builder
                    .ins()
                    .iconst(self.module.pointer_type, elem_size as i64);
                let offset = self.func.builder.ins().imul(index_wide, size_val);
                let addr = self.func.builder.ins().iadd(array_ptr, offset);
                let result_ty = ir_type_from_primitive(&expr.type_, self.module.pointer_type);
                let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
                Ok(self.func.builder.ins().load(result_ty, mflags, addr, 0))
            }
            HirExpressionKind::Ref(inner, _) => {
                match inner.as_ref() {
                    HirExpression {
                        kind: HirExpressionKind::Ident(name, _),
                        ..
                    } => match self.func.vars.get(name) {
                        Some(crate::cranelift::state::VarInfo {
                            slot: crate::cranelift::state::VarSlot::StackSlot(slot, _),
                            ..
                        }) => Ok(self.func.builder.ins().stack_addr(
                            self.module.pointer_type,
                            *slot,
                            0,
                        )),
                        _ => Err(CraneliftError::Msg(format!(
                            "cannot take reference of variable `{name}`: not stored in a stack slot"
                        ))),
                    },
                    _ => Err(CraneliftError::Msg(
                        "reference operator only supports identifiers".to_string(),
                    )),
                }
            }
            HirExpressionKind::If {
                condition,
                then_block,
                else_if,
                else_block,
                ..
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
            HirExpressionKind::Tuple(elements, _) => {
                self.compile_tuple_literal(elements, &expr.type_)
            }
            HirExpressionKind::EnumVariant {
                type_name,
                variant_index,
                payload,
                ..
            } => self.compile_enum_variant(type_name, *variant_index, payload, &expr.type_),
            HirExpressionKind::Struct {
                type_name, fields, ..
            } => self.compile_struct_literal(type_name, fields, &expr.type_),
            HirExpressionKind::FieldAccess { object, name, .. } => {
                let ptr_size = self.module.pointer_type.bytes();
                let obj = self.compile_expr(object)?;
                let offset = match &object.type_ {
                    Type::Tuple(element_types) => {
                        let index: usize = name.parse().map_err(|_| {
                            CraneliftError::Msg(format!("invalid tuple index `{name}`"))
                        })?;
                        crate::layout::tuple_field_offset(
                            index,
                            element_types,
                            self.module.types,
                            ptr_size,
                        )
                    }
                    Type::Named(type_name) => match self.module.types.get(type_name) {
                        Some(HirItemKind::Struct(s)) => {
                            let field_types: Vec<(String, Type)> = s
                                .fields
                                .iter()
                                .map(|f| (f.name.clone(), f.type_.clone()))
                                .collect();
                            let (_, field_layouts) = crate::layout::struct_layout(
                                &field_types,
                                s.repr_c,
                                self.module.types,
                                ptr_size,
                            );
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
                            crate::layout::tuple_field_offset(
                                index,
                                &t.types,
                                self.module.types,
                                ptr_size,
                            )
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
                let field_clif = ir_type_from_primitive(&expr.type_, self.module.pointer_type);
                let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
                let obj_is_ptr = is_large_aggregate(&object.type_, self.module.types, ptr_size);
                if obj_is_ptr {
                    let addr = if offset == 0 {
                        obj
                    } else {
                        let off_val = self
                            .func
                            .builder
                            .ins()
                            .iconst(self.module.pointer_type, offset as i64);
                        self.func.builder.ins().iadd(obj, off_val)
                    };
                    Ok(self.func.builder.ins().load(field_clif, mflags, addr, 0))
                } else {
                    // Small aggregate packed as i64: materialize on stack to extract fields
                    // todo: avoid temp stack slot, use bit extraction for small offsets
                    let slot = self
                        .func
                        .builder
                        .create_sized_stack_slot(StackSlotData::new(
                            StackSlotKind::ExplicitSlot,
                            8,
                            0,
                        ));
                    let base =
                        self.func
                            .builder
                            .ins()
                            .stack_addr(self.module.pointer_type, slot, 0);
                    self.func.builder.ins().store(mflags, obj, base, 0);
                    let addr = if offset == 0 {
                        base
                    } else {
                        let off_val = self
                            .func
                            .builder
                            .ins()
                            .iconst(self.module.pointer_type, offset as i64);
                        self.func.builder.ins().iadd(base, off_val)
                    };
                    Ok(self.func.builder.ins().load(field_clif, mflags, addr, 0))
                }
            }
        }
    }

    pub(super) fn compile_pow(
        &mut self,
        base_val: ir::Value,
        exp_val: ir::Value,
        exp_expr: &HirExpression,
    ) -> Result<ir::Value, CraneliftError> {
        let ty = self.func.builder.func.dfg.value_type(base_val);
        if ty == types::F64 {
            return Ok(self.compile_float_pow(base_val, exp_val));
        }
        if let HirExpressionKind::Int(value, _) = &exp_expr.kind {
            if *value < 0 {
                return Err(CraneliftError::Msg(format!(
                    "integer power with negative exponent `{value}` is not defined"
                )));
            }
        let one = self.func.builder.ins().iconst(ty, 1);
        if *value == 0 {
            return Ok(one);
        }
        if *value == 1 {
            return Ok(base_val);
        }
        if *value <= 3 {
            let mut result = one;
            for _ in 0..*value {
                result = self.func.builder.ins().imul(result, base_val);
            }
            return Ok(result);
        }
        }
        Ok(self.compile_int_pow_loop(base_val, exp_val, ty))
    }

    fn compile_int_pow_loop(&mut self, base_val: ir::Value, exp_val: ir::Value, ty: ir::Type) -> ir::Value {
        let zero = self.func.builder.ins().iconst(ty, 0);
        let one = self.func.builder.ins().iconst(ty, 1);

        let header = self.func.builder.create_block();
        let body = self.func.builder.create_block();
        let done = self.func.builder.create_block();

        self.func.builder.ins().jump(
            header,
            &[
                ir::BlockArg::from(exp_val),
                ir::BlockArg::from(base_val),
                ir::BlockArg::from(one),
            ],
        );
        self.func.builder.switch_to_block(header);
        let exp = self.func.builder.append_block_param(header, ty);
        let base = self.func.builder.append_block_param(header, ty);
        let result = self.func.builder.append_block_param(header, ty);
        let cond = self
            .func
            .builder
            .ins()
            .icmp(IntCC::SignedGreaterThan, exp, zero);
        self.func.builder.ins().brif(
            cond,
            body,
            &[
                ir::BlockArg::from(exp),
                ir::BlockArg::from(base),
                ir::BlockArg::from(result),
            ],
            done,
            &[ir::BlockArg::from(result)],
        );

        self.func.builder.switch_to_block(body);
        let body_exp = self.func.builder.append_block_param(body, ty);
        let body_base = self.func.builder.append_block_param(body, ty);
        let body_result = self.func.builder.append_block_param(body, ty);
        let odd = self.func.builder.ins().band(body_exp, one);
        let product = self.func.builder.ins().imul(body_result, body_base);
        let next_result = self.func.builder.ins().select(odd, product, body_result);
        let next_base = self.func.builder.ins().imul(body_base, body_base);
        let next_exp = self.func.builder.ins().ushr(body_exp, one);
        self.func.builder.ins().jump(
            header,
            &[
                ir::BlockArg::from(next_exp),
                ir::BlockArg::from(next_base),
                ir::BlockArg::from(next_result),
            ],
        );
        self.func.builder.seal_block(header);
        self.func.builder.seal_block(body);

        self.func.builder.switch_to_block(done);
        let done_result = self.func.builder.append_block_param(done, ty);
        self.func.builder.seal_block(done);
        done_result
    }

    fn compile_float_pow(&mut self, base_val: ir::Value, exp_val: ir::Value) -> ir::Value {
        let zero = self.func.builder.ins().f64const(0.0);
        let one = self.func.builder.ins().f64const(1.0);
        let exp_neg = self
            .func
            .builder
            .ins()
            .fcmp(FloatCC::LessThan, exp_val, zero);
        let negated = self.func.builder.ins().fsub(zero, exp_val);
        let work = self.func.builder.ins().select(exp_neg, negated, exp_val);

        let header = self.func.builder.create_block();
        let body = self.func.builder.create_block();
        let done = self.func.builder.create_block();

        self.func
            .builder
            .ins()
            .jump(header, &[ir::BlockArg::from(work), ir::BlockArg::from(one)]);
        self.func.builder.switch_to_block(header);
        let counter = self.func.builder.append_block_param(header, types::F64);
        let result = self.func.builder.append_block_param(header, types::F64);
        let cond = self
            .func
            .builder
            .ins()
            .fcmp(FloatCC::LessThanOrEqual, counter, zero);
        self.func.builder.ins().brif(
            cond,
            done,
            &[ir::BlockArg::from(result)],
            body,
            &[ir::BlockArg::from(counter), ir::BlockArg::from(result)],
        );

        self.func.builder.switch_to_block(body);
        let body_counter = self.func.builder.append_block_param(body, types::F64);
        let body_result = self.func.builder.append_block_param(body, types::F64);
        let next_counter = self.func.builder.ins().fsub(body_counter, one);
        let next_result = self.func.builder.ins().fmul(body_result, base_val);
        self.func.builder.ins().jump(
            header,
            &[ir::BlockArg::from(next_counter), ir::BlockArg::from(next_result)],
        );
        self.func.builder.seal_block(header);
        self.func.builder.seal_block(body);

        self.func.builder.switch_to_block(done);
        let done_result = self.func.builder.append_block_param(done, types::F64);
        self.func.builder.seal_block(done);

        let reciprocal = self.func.builder.ins().fdiv(one, done_result);
        self.func.builder.ins().select(exp_neg, reciprocal, done_result)
    }

    pub fn compile_expr_if(&mut self, if_expr: IfExprBundle) -> Result<ir::Value, CraneliftError> {
        let IfExprBundle {
            condition,
            then_block,
            else_if,
            else_block,
            result_type,
        } = if_expr;

        let if_header = self.func.builder.create_block();
        let then_block_id = self.func.builder.create_block();
        let else_block_id = self.func.builder.create_block();
        let merge_block_id = self.func.builder.create_block();

        let result_slot = if !matches!(result_type, Type::Primitive(Primitive::Unit)) {
            let result_type = ir_type_from_primitive(result_type, self.module.pointer_type);
            let slot = self
                .func
                .builder
                .create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    result_type.bytes(),
                    0,
                ));
            let result_ptr = self
                .func
                .builder
                .ins()
                .stack_addr(self.module.pointer_type, slot, 0);
            Some((result_type, result_ptr))
        } else {
            None
        };

        self.func.builder.ins().jump(if_header, &[]);
        self.func.builder.switch_to_block(if_header);

        let cond_val = self.compile_expr(condition)?;
        self.func
            .builder
            .ins()
            .brif(cond_val, then_block_id, &[], else_block_id, &[]);
        self.func.builder.seal_block(if_header);

        let mut if_ctx = IfBranchCtx {
            merge_block: merge_block_id,
            result_slot,
        };

        self.compile_if_branch(then_block, then_block_id, &mut if_ctx)?;
        self.compile_else_if_chain(else_if, else_block, else_block_id, &mut if_ctx)?;

        self.func.builder.switch_to_block(merge_block_id);
        self.func.builder.seal_block(merge_block_id);

        let result_val = match if_ctx.result_slot {
            Some((result_type, result_ptr)) => {
                let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
                self.func
                    .builder
                    .ins()
                    .load(result_type, mflags, result_ptr, 0)
            }
            None => self.func.builder.ins().iconst(types::I8, 0),
        };

        let after = self.func.builder.create_block();
        self.func.builder.ins().jump(after, &[]);
        self.func.builder.switch_to_block(after);
        Ok(result_val)
    }

    fn compile_if_branch(
        &mut self,
        stmts: &[HirStatement],
        block_id: ir::Block,
        if_ctx: &mut IfBranchCtx,
    ) -> Result<(), CraneliftError> {
        self.func.builder.switch_to_block(block_id);
        let mut terminated = false;
        for (i, stmt) in stmts.iter().enumerate() {
            if terminated {
                break;
            }
            if i == stmts.len() - 1
                && let HirStatement {
                    kind: HirStatementKind::Value(val_expr, _),
                    ..
                } = stmt
            {
                let val = self.compile_expr(val_expr)?;
                if let Some((_res_type, res_ptr)) = if_ctx.result_slot {
                    let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
                    self.func.builder.ins().store(mflags, val, res_ptr, 0);
                }
                self.func.builder.ins().jump(if_ctx.merge_block, &[]);
                terminated = true;
                break;
            }
            self.compile_stmt(stmt, &mut terminated)?;
        }
        if !terminated {
            self.func.builder.ins().jump(if_ctx.merge_block, &[]);
        }
        Ok(())
    }

    fn compile_else_if_chain(
        &mut self,
        else_if: &[(HirExpression, Vec<HirStatement>)],
        else_block: &Option<Vec<HirStatement>>,
        else_block_id: ir::Block,
        if_ctx: &mut IfBranchCtx,
    ) -> Result<(), CraneliftError> {
        self.func.builder.switch_to_block(else_block_id);
        for (cond, block) in else_if {
            let cond_val = self.compile_expr(cond)?;
            let inner_then = self.func.builder.create_block();
            let inner_else = self.func.builder.create_block();
            self.func
                .builder
                .ins()
                .brif(cond_val, inner_then, &[], inner_else, &[]);
            self.compile_if_branch(block, inner_then, if_ctx)?;
            self.func.builder.switch_to_block(inner_else);
        }
        if let Some(stmts) = else_block {
            let mut terminated = false;
            for (i, stmt) in stmts.iter().enumerate() {
                let is_last = i == stmts.len() - 1;
                if is_last
                    && let HirStatement {
                        kind: HirStatementKind::Value(val_expr, _),
                        ..
                    } = stmt
                {
                    let val = self.compile_expr(val_expr)?;
                    if let Some((_res_type, res_ptr)) = if_ctx.result_slot {
                        let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
                        self.func.builder.ins().store(mflags, val, res_ptr, 0);
                    }
                    self.func.builder.ins().jump(if_ctx.merge_block, &[]);
                    terminated = true;
                    break;
                }
                self.compile_stmt(stmt, &mut terminated)?;
            }
            if !terminated {
                self.func.builder.ins().jump(if_ctx.merge_block, &[]);
            }
        } else {
            self.func.builder.ins().jump(if_ctx.merge_block, &[]);
        }
        Ok(())
    }

    fn compile_tuple_literal(
        &mut self,
        elements: &[HirExpression],
        result_type: &Type,
    ) -> Result<ir::Value, CraneliftError> {
        let ptr_size = self.module.pointer_type.bytes();
        let total_size = crate::layout::size_of(result_type, self.module.types, ptr_size);
        let is_large = is_large_aggregate(result_type, self.module.types, ptr_size);
        let slot = self
            .func
            .builder
            .create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                if is_large { total_size } else { 8 },
                0,
            ));
        let base = self
            .func
            .builder
            .ins()
            .stack_addr(self.module.pointer_type, slot, 0);
        if let Type::Tuple(element_types) = result_type {
            let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
            if !is_large {
                let zero = self.func.builder.ins().iconst(types::I64, 0);
                self.func.builder.ins().store(mflags, zero, base, 0);
            }
            for (i, element) in elements.iter().enumerate() {
                let val = self.compile_expr(element)?;
                let offset = crate::layout::tuple_field_offset(
                    i,
                    element_types,
                    self.module.types,
                    ptr_size,
                );
                let addr = if offset == 0 {
                    base
                } else {
                    let off_val = self
                        .func
                        .builder
                        .ins()
                        .iconst(self.module.pointer_type, offset as i64);
                    self.func.builder.ins().iadd(base, off_val)
                };
                self.func.builder.ins().store(mflags, val, addr, 0);
            }
        }
        if is_large {
            Ok(base)
        } else {
            let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
            Ok(self.func.builder.ins().load(types::I64, mflags, base, 0))
        }
    }

    fn compile_enum_variant(
        &mut self,
        type_name: &str,
        variant_index: usize,
        payload: &[HirExpression],
        result_type: &Type,
    ) -> Result<ir::Value, CraneliftError> {
        let ptr_size = self.module.pointer_type.bytes();
        let hir_enum = match self.module.types.get(type_name) {
            Some(HirItemKind::Enum(e)) => e,
            _ => {
                return Err(CraneliftError::Msg(format!(
                    "type `{type_name}` is not an enum"
                )));
            }
        };
        // todo: niche/discriminant overlap optimization for Option-like enums
        let all_variant_data: Vec<Vec<Type>> = hir_enum
            .variants
            .iter()
            .map(|v| match &v.data {
                Some(HirEnumVariantData::Tuple(types)) => types.clone(),
                Some(HirEnumVariantData::Struct(fields)) => {
                    fields.iter().map(|f| f.type_.clone()).collect()
                }
                None => Vec::new(),
            })
            .collect();
        let (enum_total_size, data_offset, _disc_size) =
            crate::layout::enum_layout(&all_variant_data, self.module.types, ptr_size);
        let is_large = is_large_aggregate(result_type, self.module.types, ptr_size);
        let slot = self
            .func
            .builder
            .create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                if is_large { enum_total_size } else { 8 },
                0,
            ));
        let base = self
            .func
            .builder
            .ins()
            .stack_addr(self.module.pointer_type, slot, 0);
        let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
        if !is_large {
            let zero = self.func.builder.ins().iconst(types::I64, 0);
            self.func.builder.ins().store(mflags, zero, base, 0);
        }
        let disc_val = self
            .func
            .builder
            .ins()
            .iconst(types::I8, variant_index as i64);
        self.func.builder.ins().store(mflags, disc_val, base, 0);
        let variant = &hir_enum.variants[variant_index];
        let payload_types: Vec<Type> = match &variant.data {
            Some(HirEnumVariantData::Tuple(types)) => types.clone(),
            Some(HirEnumVariantData::Struct(fields)) => {
                fields.iter().map(|f| f.type_.clone()).collect()
            }
            None => Vec::new(),
        };
        let mut data_offset_acc = 0u32;
        for (i, elem) in payload.iter().enumerate() {
            let val = self.compile_expr(elem)?;
            let elem_align =
                crate::layout::align_of(&payload_types[i], self.module.types, ptr_size);
            data_offset_acc = crate::layout::align_up(data_offset_acc, elem_align);
            let field_addr = if data_offset + data_offset_acc == 0 {
                base
            } else {
                let off = self.func.builder.ins().iconst(
                    self.module.pointer_type,
                    (data_offset + data_offset_acc) as i64,
                );
                self.func.builder.ins().iadd(base, off)
            };
            self.func.builder.ins().store(mflags, val, field_addr, 0);
            data_offset_acc +=
                crate::layout::size_of(&payload_types[i], self.module.types, ptr_size);
        }
        if is_large {
            Ok(base)
        } else {
            Ok(self.func.builder.ins().load(types::I64, mflags, base, 0))
        }
    }

    fn compile_struct_literal(
        &mut self,
        type_name: &str,
        fields: &[(String, HirExpression)],
        result_type: &Type,
    ) -> Result<ir::Value, CraneliftError> {
        let ptr_size = self.module.pointer_type.bytes();
        let hir_struct = match self.module.types.get(type_name) {
            Some(HirItemKind::Struct(s)) => s,
            _ => {
                return Err(CraneliftError::Msg(format!(
                    "type `{type_name}` is not a struct"
                )));
            }
        };
        let field_types: Vec<(String, Type)> = hir_struct
            .fields
            .iter()
            .map(|f| (f.name.clone(), f.type_.clone()))
            .collect();
        let (total_size, field_layouts) = crate::layout::struct_layout(
            &field_types,
            hir_struct.repr_c,
            self.module.types,
            ptr_size,
        );
        let is_large = is_large_aggregate(result_type, self.module.types, ptr_size);
        let slot = self
            .func
            .builder
            .create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                if is_large { total_size } else { 8 },
                0,
            ));
        let base = self
            .func
            .builder
            .ins()
            .stack_addr(self.module.pointer_type, slot, 0);
        // todo: double-copy for large aggregates, use destination-passing to avoid temp alloc
        if !is_large {
            let zero = self.func.builder.ins().iconst(types::I64, 0);
            let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
            self.func.builder.ins().store(mflags, zero, base, 0);
        }
        let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
        for (name, expr) in fields {
            let val = self.compile_expr(expr)?;
            let field_idx = hir_struct
                .fields
                .iter()
                .position(|f| f.name == *name)
                .ok_or_else(|| {
                    CraneliftError::Msg(format!("struct `{type_name}` has no field `{name}`"))
                })?;
            let offset = field_layouts[field_idx].1.offset;
            let addr = if offset == 0 {
                base
            } else {
                let off_val = self
                    .func
                    .builder
                    .ins()
                    .iconst(self.module.pointer_type, offset as i64);
                self.func.builder.ins().iadd(base, off_val)
            };
            self.func.builder.ins().store(mflags, val, addr, 0);
        }
        if is_large {
            // todo: main return capped at 64-bit, trampoline shim for larger
            Ok(base)
        } else {
            Ok(self.func.builder.ins().load(types::I64, mflags, base, 0))
        }
    }

    pub(super) fn emit_memcpy(
        &mut self,
        dest: ir::Value,
        src: ir::Value,
        size: u32,
    ) -> Result<(), CraneliftError> {
        // todo: memcpy for all, inline field copies for 9-32B range
        let pointer_type = self.module.pointer_type;
        if size == 0 {
            return Ok(());
        }
        let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
        for byte_offset in 0..size {
            let off_val = self
                .func
                .builder
                .ins()
                .iconst(pointer_type, byte_offset as i64);
            let src_addr = self.func.builder.ins().iadd(src, off_val);
            let val = self.func.builder.ins().load(types::I8, mflags, src_addr, 0);
            let dest_addr = self.func.builder.ins().iadd(dest, off_val);
            self.func.builder.ins().store(mflags, val, dest_addr, 0);
        }
        Ok(())
    }
}
