use cranelift_codegen::ir::{
    self, InstBuilder, StackSlotData, StackSlotKind,
    condcodes::{FloatCC, IntCC},
    types,
};
use cranelift_module::Module;

use vinyl_parser::ast::operator::{BinaryOp, UnaryOp};
use vinyl_parser::ast::types::Primitive;
use vinyl_typecheck::hir::{
    HirEnumVariantData, HirExpression, HirExpressionKind, HirItemKind, HirMatchArm, HirPattern,
    HirPatternKind, HirStatement, HirStatementKind, LiteralValue, Type,
};

use super::state::{CodegenCtx, VarInfo, VarSlot};
use super::types::{ir_type_from_primitive, is_large_aggregate, type_needs_custom_equality};
use super::variable::{build_var_info, var_mode};
use crate::CraneliftError;

fn is_unsigned_type(t: &Type) -> bool {
    matches!(
        t,
        Type::Primitive(
            Primitive::UInt8
                | Primitive::UInt16
                | Primitive::UInt32
                | Primitive::UInt64
                | Primitive::UInt128
                | Primitive::USize
        )
    )
}

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
                if ty == types::F32 {
                    Ok(self.func.builder.ins().f32const(*v as f32))
                } else if ty == types::F64 {
                    Ok(self.func.builder.ins().f64const(*v as f64))
                } else {
                    Ok(self.emit_iconst(ty, *v))
                }
            }
            HirExpressionKind::UInt(v, _) => {
                let ty = ir_type_from_primitive(&expr.type_, self.module.pointer_type);
                Ok(self.emit_uint_const(ty, *v))
            }
            HirExpressionKind::Float(v, _) => {
                let ty = ir_type_from_primitive(&expr.type_, self.module.pointer_type);
                if ty == types::F32 {
                    Ok(self.func.builder.ins().f32const(*v as f32))
                } else {
                    Ok(self.func.builder.ins().f64const(*v))
                }
            }
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
                let ptr_size = self.module.pointer_type.bytes();
                let left_val = self.compile_expr(left)?;
                let right_val = self.compile_expr(right)?;
                let is_float = matches!(
                    left.type_,
                    Type::Primitive(Primitive::Float32 | Primitive::Float64)
                );
                Ok(match op {
                    BinaryOp::Add => {
                        if is_float {
                            self.func.builder.ins().fadd(left_val, right_val)
                        } else {
                            self.func.builder.ins().iadd(left_val, right_val)
                        }
                    }
                    BinaryOp::Sub => {
                        if is_float {
                            self.func.builder.ins().fsub(left_val, right_val)
                        } else {
                            self.func.builder.ins().isub(left_val, right_val)
                        }
                    }
                    BinaryOp::Mul => {
                        if is_float {
                            self.func.builder.ins().fmul(left_val, right_val)
                        } else {
                            self.func.builder.ins().imul(left_val, right_val)
                        }
                    }
                    BinaryOp::Div => {
                        if is_float {
                            self.func.builder.ins().fdiv(left_val, right_val)
                        } else {
                            let ty = self.func.builder.func.dfg.value_type(left_val);
                            self.emit_div_rem(
                                left_val,
                                right_val,
                                ty,
                                !is_unsigned_type(&left.type_),
                            )
                            .0
                        }
                    }
                    BinaryOp::Rem => {
                        if is_float {
                            self.func.builder.ins().srem(left_val, right_val)
                        } else {
                            let ty = self.func.builder.func.dfg.value_type(left_val);
                            self.emit_div_rem(
                                left_val,
                                right_val,
                                ty,
                                !is_unsigned_type(&left.type_),
                            )
                            .1
                        }
                    }
                    BinaryOp::Eq | BinaryOp::Ne => {
                        let eq = if is_float {
                            self.func
                                .builder
                                .ins()
                                .fcmp(FloatCC::Equal, left_val, right_val)
                        } else if type_needs_custom_equality(&left.type_, self.module.types) {
                            self.emit_structural_equality(left_val, right_val, &left.type_)?
                        } else if is_large_aggregate(&left.type_, self.module.types, ptr_size) {
                            let size =
                                crate::layout::size_of(&left.type_, self.module.types, ptr_size);
                            let diff = self.emit_memcmp_diff(left_val, right_val, size);
                            let zero = self.func.builder.ins().iconst(types::I8, 0);
                            self.func.builder.ins().icmp(IntCC::Equal, diff, zero)
                        } else {
                            self.func
                                .builder
                                .ins()
                                .icmp(IntCC::Equal, left_val, right_val)
                        };
                        if matches!(op, BinaryOp::Ne) {
                            let one = self.func.builder.ins().iconst(types::I8, 1);
                            self.func.builder.ins().bxor(eq, one)
                        } else {
                            eq
                        }
                    }
                    BinaryOp::Lt => {
                        if is_float {
                            self.func
                                .builder
                                .ins()
                                .fcmp(FloatCC::LessThan, left_val, right_val)
                        } else {
                            self.func
                                .builder
                                .ins()
                                .icmp(IntCC::SignedLessThan, left_val, right_val)
                        }
                    }
                    BinaryOp::Gt => {
                        if is_float {
                            self.func
                                .builder
                                .ins()
                                .fcmp(FloatCC::GreaterThan, left_val, right_val)
                        } else {
                            self.func.builder.ins().icmp(
                                IntCC::SignedGreaterThan,
                                left_val,
                                right_val,
                            )
                        }
                    }
                    BinaryOp::Le => {
                        if is_float {
                            self.func.builder.ins().fcmp(
                                FloatCC::LessThanOrEqual,
                                left_val,
                                right_val,
                            )
                        } else {
                            self.func.builder.ins().icmp(
                                IntCC::SignedLessThanOrEqual,
                                left_val,
                                right_val,
                            )
                        }
                    }
                    BinaryOp::Ge => {
                        if is_float {
                            self.func.builder.ins().fcmp(
                                FloatCC::GreaterThanOrEqual,
                                left_val,
                                right_val,
                            )
                        } else {
                            self.func.builder.ins().icmp(
                                IntCC::SignedGreaterThanOrEqual,
                                left_val,
                                right_val,
                            )
                        }
                    }
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
                    BinaryOp::Shr => {
                        if is_unsigned_type(&left.type_) {
                            self.func.builder.ins().ushr(left_val, right_val)
                        } else {
                            self.func.builder.ins().sshr(left_val, right_val)
                        }
                    }
                    BinaryOp::FloorDiv => {
                        let ty = self.func.builder.func.dfg.value_type(left_val);
                        let signed = !is_unsigned_type(&left.type_);
                        let (q, r) = self.emit_div_rem(left_val, right_val, ty, signed);
                        if !signed {
                            q
                        } else {
                            let zero = self.emit_iconst(ty, 0);
                            let one = self.emit_iconst(ty, 1);
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
                        let zero = self.emit_iconst(ty, 0);
                        self.func.builder.ins().isub(zero, val)
                    }
                    UnaryOp::Not => {
                        let ty = self.func.builder.func.dfg.value_type(val);
                        let one = self.emit_iconst(ty, 1);
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
                        let needs_sret = crate::layout::is_aggregate(callee_ret_type)
                            && crate::layout::aggregate_register_count(
                                callee_ret_type,
                                self.module.types,
                                ptr_size,
                            ) == 0;
                        let sret_slot = if needs_sret {
                            let slot =
                                self.func
                                    .builder
                                    .create_sized_stack_slot(StackSlotData::new(
                                        StackSlotKind::ExplicitSlot,
                                        crate::layout::size_of(
                                            callee_ret_type,
                                            self.module.types,
                                            ptr_size,
                                        ),
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
                        let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
                        for (i, arg) in args.iter().enumerate() {
                            let param_type = &callee_params[i].type_;
                            let val = self.compile_expr(arg)?;
                            if crate::layout::is_aggregate(param_type) {
                                match crate::layout::aggregate_register_count(
                                    param_type,
                                    self.module.types,
                                    ptr_size,
                                ) {
                                    2 => {
                                        let chunk0 = self.func.builder.ins().load(
                                            types::I64,
                                            mflags,
                                            val,
                                            0,
                                        );
                                        let addr1 = {
                                            let off = self
                                                .func
                                                .builder
                                                .ins()
                                                .iconst(self.module.pointer_type, 8);
                                            self.func.builder.ins().iadd(val, off)
                                        };
                                        let chunk1 = self.func.builder.ins().load(
                                            types::I64,
                                            mflags,
                                            addr1,
                                            0,
                                        );
                                        call_args.push(chunk0);
                                        call_args.push(chunk1);
                                    }
                                    0 => call_args.push(val),
                                    _ => call_args.push(val),
                                }
                            } else {
                                call_args.push(val);
                            }
                        }
                        let sig = self
                            .module
                            .module
                            .declare_func_in_func(*callee_id, self.func.builder.func);
                        let inst = self.func.builder.ins().call(sig, &call_args);
                        let result_values: Vec<ir::Value> =
                            self.func.builder.inst_results(inst).to_vec();
                        if let Some((_slot, addr)) = sret_slot {
                            Ok(addr)
                        } else if crate::layout::is_aggregate(callee_ret_type)
                            && crate::layout::aggregate_register_count(
                                callee_ret_type,
                                self.module.types,
                                ptr_size,
                            ) == 2
                        {
                            let slot =
                                self.func
                                    .builder
                                    .create_sized_stack_slot(StackSlotData::new(
                                        StackSlotKind::ExplicitSlot,
                                        crate::layout::aggregate_slot_size(
                                            callee_ret_type,
                                            self.module.types,
                                            ptr_size,
                                        ),
                                        0,
                                    ));
                            let addr = self.func.builder.ins().stack_addr(
                                self.module.pointer_type,
                                slot,
                                0,
                            );
                            self.func
                                .builder
                                .ins()
                                .store(mflags, result_values[0], addr, 0);
                            let addr1 = {
                                let off =
                                    self.func.builder.ins().iconst(self.module.pointer_type, 8);
                                self.func.builder.ins().iadd(addr, off)
                            };
                            self.func
                                .builder
                                .ins()
                                .store(mflags, result_values[1], addr1, 0);
                            Ok(addr)
                        } else if result_values.is_empty() {
                            Ok(self.func.builder.ins().iconst(types::I32, 0))
                        } else {
                            Ok(result_values[0])
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
                let elem_size = crate::layout::array_element_stride(
                    element_type,
                    self.module.types,
                    self.module.pointer_type.bytes(),
                );
                let slot = self
                    .func
                    .builder
                    .create_sized_stack_slot(StackSlotData::new(
                        StackSlotKind::ExplicitSlot,
                        crate::layout::size_of(
                            &expr.type_,
                            self.module.types,
                            self.module.pointer_type.bytes(),
                        ),
                        0,
                    ));
                let base = self
                    .func
                    .builder
                    .ins()
                    .stack_addr(self.module.pointer_type, slot, 0);
                self.zero_slot(
                    base,
                    crate::layout::size_of(
                        &expr.type_,
                        self.module.types,
                        self.module.pointer_type.bytes(),
                    ),
                );
                for (i, element) in elements.iter().enumerate() {
                    let val = self.compile_expr(element)?;
                    let offset = self
                        .func
                        .builder
                        .ins()
                        .iconst(self.module.pointer_type, (i as i64) * (elem_size as i64));
                    let addr = self.func.builder.ins().iadd(base, offset);
                    self.store_by_value(element_type, val, addr)?;
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
                let elem_size = crate::layout::array_element_stride(
                    &expr.type_,
                    self.module.types,
                    self.module.pointer_type.bytes(),
                );
                let size_val = self
                    .func
                    .builder
                    .ins()
                    .iconst(self.module.pointer_type, elem_size as i64);
                let offset = self.func.builder.ins().imul(index_wide, size_val);
                let addr = self.func.builder.ins().iadd(array_ptr, offset);
                if crate::layout::is_aggregate(&expr.type_) {
                    let chunks = crate::layout::aggregate_register_count(
                        &expr.type_,
                        self.module.types,
                        self.module.pointer_type.bytes(),
                    );
                    if chunks != 1 {
                        // 9+ bytes: value lives in memory, hand back the element address
                        return Ok(addr);
                    }
                    let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
                    return Ok(self.func.builder.ins().load(types::I64, mflags, addr, 0));
                }
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
            HirExpressionKind::Match {
                span: _,
                value,
                arms,
            } => self.compile_expr_match(value, arms, &expr.type_),
            HirExpressionKind::FieldAccess { object, name, .. } => {                let ptr_size = self.module.pointer_type.bytes();
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
                    Type::Named(type_name) => match crate::layout::resolve_type_item(
                        type_name,
                        self.module.types,
                        &mut Vec::new(),
                    ) {
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
                    self.load_field_value(addr, &expr.type_)
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
                    self.load_field_value(addr, &expr.type_)
                }
            }
        }
    }

    /// Load a field value given its address. Primitive fields load their CLIF
    /// type; aggregates <=8 bytes load the packed i64; larger aggregates stay
    /// in memory and hand back their address.
    fn load_field_value(
        &mut self,
        addr: ir::Value,
        field_type: &Type,
    ) -> Result<ir::Value, CraneliftError> {
        let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
        if crate::layout::is_aggregate(field_type) {
            let chunks = crate::layout::aggregate_register_count(
                field_type,
                self.module.types,
                self.module.pointer_type.bytes(),
            );
            if chunks == 1 {
                return Ok(self.func.builder.ins().load(types::I64, mflags, addr, 0));
            }
            return Ok(addr);
        }
        let field_clif = ir_type_from_primitive(field_type, self.module.pointer_type);
        Ok(self.func.builder.ins().load(field_clif, mflags, addr, 0))
    }

    pub(super) fn compile_pow(
        &mut self,
        base_val: ir::Value,
        exp_val: ir::Value,
        exp_expr: &HirExpression,
    ) -> Result<ir::Value, CraneliftError> {
        let ty = self.func.builder.func.dfg.value_type(base_val);
        if ty == types::F32 || ty == types::F64 {
            return Ok(self.compile_float_pow(base_val, exp_val, ty));
        }
        if let HirExpressionKind::Int(value, _) = &exp_expr.kind {
            if *value < 0 {
                return Err(CraneliftError::Msg(format!(
                    "integer power with negative exponent `{value}` is not defined"
                )));
            }
            let one = self.emit_iconst(ty, 1);
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

    fn compile_int_pow_loop(
        &mut self,
        base_val: ir::Value,
        exp_val: ir::Value,
        ty: ir::Type,
    ) -> ir::Value {
        let zero = self.emit_iconst(ty, 0);
        let one = self.emit_iconst(ty, 1);

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

    fn float_const(&mut self, ty: ir::Type, value: f64) -> ir::Value {
        if ty == types::F32 {
            self.func.builder.ins().f32const(value as f32)
        } else {
            self.func.builder.ins().f64const(value)
        }
    }

    /// Build an integer constant, handling 128-bit values (which have no
    /// single-immediate form) via `iconcat` of two 64-bit halves.
    fn emit_iconst(&mut self, ty: ir::Type, value: i128) -> ir::Value {
        if ty == types::I128 {
            let lo = self
                .func
                .builder
                .ins()
                .iconst(types::I64, value as u64 as i64);
            let hi = self
                .func
                .builder
                .ins()
                .iconst(types::I64, (value >> 64) as u64 as i64);
            self.func.builder.ins().iconcat(lo, hi)
        } else {
            self.func.builder.ins().iconst(ty, value as i64)
        }
    }

    /// Build an unsigned integer constant, handling 128-bit values.
    fn emit_uint_const(&mut self, ty: ir::Type, value: u128) -> ir::Value {
        if ty == types::I128 {
            let lo = self
                .func
                .builder
                .ins()
                .iconst(types::I64, value as u64 as i64);
            let hi = self
                .func
                .builder
                .ins()
                .iconst(types::I64, (value >> 64) as u64 as i64);
            self.func.builder.ins().iconcat(lo, hi)
        } else {
            self.func.builder.ins().iconst(ty, value as i64)
        }
    }

    /// Divide and remainder. 128-bit division has no cranelift lowering, so it
    /// uses a shift-subtract long-division loop with the same trap semantics as
    /// the native 64-bit instructions: div-by-zero and signed MIN / -1 overflow.
    pub(super) fn emit_div_rem(
        &mut self,
        dividend: ir::Value,
        divisor: ir::Value,
        ty: ir::Type,
        signed: bool,
    ) -> (ir::Value, ir::Value) {
        if ty != types::I128 {
            if signed {
                return (
                    self.func.builder.ins().sdiv(dividend, divisor),
                    self.func.builder.ins().srem(dividend, divisor),
                );
            }
            return (
                self.func.builder.ins().udiv(dividend, divisor),
                self.func.builder.ins().urem(dividend, divisor),
            );
        }

        let zero = self.emit_iconst(ty, 0);
        let is_zero = self.func.builder.ins().icmp(IntCC::Equal, divisor, zero);
        self.func
            .builder
            .ins()
            .trapnz(is_zero, ir::TrapCode::INTEGER_DIVISION_BY_ZERO);

        if !signed {
            return self.emit_udiv_rem_loop(dividend, divisor, ty);
        }

        let min = self.emit_iconst(ty, i128::MIN);
        let neg_one = self.emit_iconst(ty, -1);
        let is_min = self.func.builder.ins().icmp(IntCC::Equal, dividend, min);
        let is_neg_one = self.func.builder.ins().icmp(IntCC::Equal, divisor, neg_one);
        let overflows = self.func.builder.ins().band(is_min, is_neg_one);
        self.func
            .builder
            .ins()
            .trapnz(overflows, ir::TrapCode::INTEGER_OVERFLOW);

        let dividend_neg = self
            .func
            .builder
            .ins()
            .icmp(IntCC::SignedLessThan, dividend, zero);
        let divisor_neg = self
            .func
            .builder
            .ins()
            .icmp(IntCC::SignedLessThan, divisor, zero);
        let neg_dividend = self.func.builder.ins().isub(zero, dividend);
        let neg_divisor = self.func.builder.ins().isub(zero, divisor);
        let dividend_mag = self
            .func
            .builder
            .ins()
            .select(dividend_neg, neg_dividend, dividend);
        let divisor_mag = self
            .func
            .builder
            .ins()
            .select(divisor_neg, neg_divisor, divisor);

        let (quotient_mag, remainder_mag) = self.emit_udiv_rem_loop(dividend_mag, divisor_mag, ty);
        let quotient_neg = self.func.builder.ins().bxor(dividend_neg, divisor_neg);
        let neg_quotient = self.func.builder.ins().isub(zero, quotient_mag);
        let quotient = self
            .func
            .builder
            .ins()
            .select(quotient_neg, neg_quotient, quotient_mag);
        let neg_remainder = self.func.builder.ins().isub(zero, remainder_mag);
        let remainder = self
            .func
            .builder
            .ins()
            .select(dividend_neg, neg_remainder, remainder_mag);
        (quotient, remainder)
    }

    /// Unsigned 128-bit long division: 128 iterations of shift-subtract. All
    /// operations (shift, and, or, compare, subtract, select) are natively
    /// lowered for I128.
    fn emit_udiv_rem_loop(
        &mut self,
        dividend: ir::Value,
        divisor: ir::Value,
        ty: ir::Type,
    ) -> (ir::Value, ir::Value) {
        let zero = self.emit_iconst(ty, 0);
        let one = self.emit_iconst(ty, 1);
        let bit_127 = self.emit_iconst(ty, 127);
        let iterations = self.emit_iconst(ty, 128);

        let header = self.func.builder.create_block();
        let body = self.func.builder.create_block();
        let done = self.func.builder.create_block();

        self.func.builder.ins().jump(
            header,
            &[
                ir::BlockArg::from(iterations),
                ir::BlockArg::from(dividend),
                ir::BlockArg::from(zero),
                ir::BlockArg::from(zero),
            ],
        );
        self.func.builder.switch_to_block(header);
        let counter = self.func.builder.append_block_param(header, ty);
        let shifter = self.func.builder.append_block_param(header, ty);
        let remainder = self.func.builder.append_block_param(header, ty);
        let quotient = self.func.builder.append_block_param(header, ty);
        let done_cond = self.func.builder.ins().icmp(IntCC::Equal, counter, zero);
        self.func.builder.ins().brif(
            done_cond,
            done,
            &[ir::BlockArg::from(quotient), ir::BlockArg::from(remainder)],
            body,
            &[
                ir::BlockArg::from(counter),
                ir::BlockArg::from(shifter),
                ir::BlockArg::from(remainder),
                ir::BlockArg::from(quotient),
            ],
        );

        self.func.builder.switch_to_block(body);
        let body_counter = self.func.builder.append_block_param(body, ty);
        let body_shifter = self.func.builder.append_block_param(body, ty);
        let body_remainder = self.func.builder.append_block_param(body, ty);
        let body_quotient = self.func.builder.append_block_param(body, ty);

        let shifted = self.func.builder.ins().ushr(body_shifter, bit_127);
        let msb = self.func.builder.ins().band(shifted, one);
        let shifted_rem = self.func.builder.ins().ishl(body_remainder, one);
        let next_remainder = self.func.builder.ins().bor(shifted_rem, msb);
        let next_shifter = self.func.builder.ins().ishl(body_shifter, one);
        let next_quotient = self.func.builder.ins().ishl(body_quotient, one);
        let ge = self.func.builder.ins().icmp(
            IntCC::UnsignedGreaterThanOrEqual,
            next_remainder,
            divisor,
        );
        let remainder_minus = self.func.builder.ins().isub(next_remainder, divisor);
        let final_remainder = self
            .func
            .builder
            .ins()
            .select(ge, remainder_minus, next_remainder);
        let quotient_or_one = self.func.builder.ins().bor(next_quotient, one);
        let final_quotient = self
            .func
            .builder
            .ins()
            .select(ge, quotient_or_one, next_quotient);
        let next_counter = self.func.builder.ins().isub(body_counter, one);
        self.func.builder.ins().jump(
            header,
            &[
                ir::BlockArg::from(next_counter),
                ir::BlockArg::from(next_shifter),
                ir::BlockArg::from(final_remainder),
                ir::BlockArg::from(final_quotient),
            ],
        );
        self.func.builder.seal_block(header);
        self.func.builder.seal_block(body);

        self.func.builder.switch_to_block(done);
        let done_quotient = self.func.builder.append_block_param(done, ty);
        let done_remainder = self.func.builder.append_block_param(done, ty);
        self.func.builder.seal_block(done);
        (done_quotient, done_remainder)
    }

    fn compile_float_pow(
        &mut self,
        base_val: ir::Value,
        exp_val: ir::Value,
        ty: ir::Type,
    ) -> ir::Value {
        let zero = self.float_const(ty, 0.0);
        let one = self.float_const(ty, 1.0);
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
        let counter = self.func.builder.append_block_param(header, ty);
        let result = self.func.builder.append_block_param(header, ty);
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
        let body_counter = self.func.builder.append_block_param(body, ty);
        let body_result = self.func.builder.append_block_param(body, ty);
        let next_counter = self.func.builder.ins().fsub(body_counter, one);
        let next_result = self.func.builder.ins().fmul(body_result, base_val);
        self.func.builder.ins().jump(
            header,
            &[
                ir::BlockArg::from(next_counter),
                ir::BlockArg::from(next_result),
            ],
        );
        self.func.builder.seal_block(header);
        self.func.builder.seal_block(body);

        self.func.builder.switch_to_block(done);
        let done_result = self.func.builder.append_block_param(done, ty);
        self.func.builder.seal_block(done);

        let reciprocal = self.func.builder.ins().fdiv(one, done_result);
        self.func
            .builder
            .ins()
            .select(exp_neg, reciprocal, done_result)
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
        let is_large = is_large_aggregate(result_type, self.module.types, ptr_size);
        let slot = self
            .func
            .builder
            .create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                crate::layout::aggregate_slot_size(result_type, self.module.types, ptr_size),
                0,
            ));
        let base = self
            .func
            .builder
            .ins()
            .stack_addr(self.module.pointer_type, slot, 0);
        if let Type::Tuple(element_types) = result_type {
            self.zero_slot(
                base,
                crate::layout::size_of(result_type, self.module.types, ptr_size),
            );
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
                self.store_by_value(&element_types[i], val, addr)?;
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
        let hir_enum =
            match crate::layout::resolve_type_item(type_name, self.module.types, &mut Vec::new()) {
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
        let (_, data_offset, _disc_size) =
            crate::layout::enum_layout(&all_variant_data, self.module.types, ptr_size);
        let is_large = is_large_aggregate(result_type, self.module.types, ptr_size);
        let slot = self
            .func
            .builder
            .create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                crate::layout::aggregate_slot_size(result_type, self.module.types, ptr_size),
                0,
            ));
        let base = self
            .func
            .builder
            .ins()
            .stack_addr(self.module.pointer_type, slot, 0);
        let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
        self.zero_slot(
            base,
            crate::layout::size_of(result_type, self.module.types, ptr_size),
        );
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
            self.store_by_value(&payload_types[i], val, field_addr)?;
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
        let hir_struct =
            match crate::layout::resolve_type_item(type_name, self.module.types, &mut Vec::new()) {
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
        let (_, field_layouts) = crate::layout::struct_layout(
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
                crate::layout::aggregate_slot_size(result_type, self.module.types, ptr_size),
                0,
            ));
        let base = self
            .func
            .builder
            .ins()
            .stack_addr(self.module.pointer_type, slot, 0);
        // todo: double-copy for large aggregates, use destination-passing to avoid temp alloc
        self.zero_slot(
            base,
            crate::layout::size_of(result_type, self.module.types, ptr_size),
        );
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
            let field_type = &field_types[field_idx].1;
            self.store_by_value(field_type, val, addr)?;
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

    /// Zero the first `size` bytes at `base` so padding bytes in aggregate
    /// literal slots are deterministic (equality memcmp and slot copies rely
    /// on them being stable).
    pub(super) fn zero_slot(&mut self, base: ir::Value, size: u32) {
        let pointer_type = self.module.pointer_type;
        let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
        let mut offset = 0u32;
        while offset + 8 <= size {
            let addr = if offset == 0 {
                base
            } else {
                let off_val = self.func.builder.ins().iconst(pointer_type, offset as i64);
                self.func.builder.ins().iadd(base, off_val)
            };
            let zero = self.func.builder.ins().iconst(types::I64, 0);
            self.func.builder.ins().store(mflags, zero, addr, 0);
            offset += 8;
        }
        if offset < size {
            let addr = if offset == 0 {
                base
            } else {
                let off_val = self.func.builder.ins().iconst(pointer_type, offset as i64);
                self.func.builder.ins().iadd(base, off_val)
            };
            let zero = self.func.builder.ins().iconst(types::I8, 0);
            self.func.builder.ins().store(mflags, zero, addr, 0);
        }
    }

    /// Store a value into memory, copying by-value when the type is an
    /// aggregate larger than a single register chunk (value is a pointer).
    pub(super) fn store_by_value(
        &mut self,
        ty: &Type,
        val: ir::Value,
        addr: ir::Value,
    ) -> Result<(), CraneliftError> {
        if crate::layout::is_aggregate(ty)
            && crate::layout::aggregate_register_count(
                ty,
                self.module.types,
                self.module.pointer_type.bytes(),
            ) != 1
        {
            let ptr_size = self.module.pointer_type.bytes();
            self.emit_memcpy(
                addr,
                val,
                crate::layout::aggregate_copy_size(ty, self.module.types, ptr_size),
            )
        } else {
            let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
            self.func.builder.ins().store(mflags, val, addr, 0);
            Ok(())
        }
    }

    fn store_tmp(&mut self, value: ir::Value, bytes: u32) -> Result<ir::Value, CraneliftError> {
        let slot = self.func.builder.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            bytes,
            0,
        ));
        let addr = self
            .func
            .builder
            .ins()
            .stack_addr(self.module.pointer_type, slot, 0);
        let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
        self.func.builder.ins().store(mflags, value, addr, 0);
        Ok(addr)
    }

    /// Load the value an expression of type `ty` would carry when read from
    /// `addr`: aggregate elements >8 bytes come back as their address, packed
    /// small ones as an i64, scalars by their primitive type. Arrays are always
    /// memory-backed, so they come back as their address.
    fn load_value_of_type(
        &mut self,
        addr: ir::Value,
        ty: &Type,
    ) -> Result<ir::Value, CraneliftError> {
        let ptr_size = self.module.pointer_type.bytes();
        let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
        if matches!(ty, Type::Array { .. }) {
            return Ok(addr);
        }
        if crate::layout::is_aggregate(ty) {
            let chunks =
                crate::layout::aggregate_register_count(ty, self.module.types, ptr_size);
            if chunks != 1 {
                return Ok(addr);
            }
            return Ok(self.func.builder.ins().load(types::I64, mflags, addr, 0));
        }
        let result_ty = ir_type_from_primitive(ty, self.module.pointer_type);
        Ok(self.func.builder.ins().load(result_ty, mflags, addr, 0))
    }

    /// Field-wise equality for aggregates that cannot be compared by raw bytes:
    /// float leaves use IEEE fcmp (so NaN != NaN and -0.0 == 0.0), and arrays
    /// are walked element-wise because their values are addresses. Both operands
    /// must be the same kind of value `compile_expr` produces for `ty` (i64 for
    /// packed small aggregates, an address otherwise).
    fn emit_structural_equality(
        &mut self,
        left: ir::Value,
        right: ir::Value,
        ty: &Type,
    ) -> Result<ir::Value, CraneliftError> {
        let ptr_size = self.module.pointer_type.bytes();
        let packed_small = crate::layout::is_aggregate(ty)
            && crate::layout::size_of(ty, self.module.types, ptr_size) <= 8
            && !matches!(ty, Type::Array { .. });
        let (left, right) = if packed_small {
            (self.store_tmp(left, 8)?, self.store_tmp(right, 8)?)
        } else {
            (left, right)
        };
        let one = self.func.builder.ins().iconst(types::I8, 1);
        Ok(match ty {
            Type::Primitive(Primitive::Float32 | Primitive::Float64) => {
                self.func.builder.ins().fcmp(FloatCC::Equal, left, right)
            }
            Type::Primitive(Primitive::Unit) => one,
            Type::Primitive(_) => self.func.builder.ins().icmp(IntCC::Equal, left, right),
            Type::Array { element, size } => {
                let stride =
                    crate::layout::array_element_stride(element, self.module.types, ptr_size);
                let mut acc = one;
                for index in 0..*size {
                    let offset = (index as u32) * stride;
                    let left_addr = self.addr_at(left, offset);
                    let right_addr = self.addr_at(right, offset);
                    let left_elem = self.load_value_of_type(left_addr, element)?;
                    let right_elem = self.load_value_of_type(right_addr, element)?;
                    let elem_eq =
                        self.emit_structural_equality(left_elem, right_elem, element)?;
                    acc = self.func.builder.ins().band(acc, elem_eq);
                }
                acc
            }
            Type::Tuple(elements) => {
                let mut acc = one;
                for (index, element) in elements.iter().enumerate() {
                    let offset = crate::layout::tuple_field_offset(
                        index,
                        elements,
                        self.module.types,
                        ptr_size,
                    );
                    let left_addr = self.addr_at(left, offset);
                    let right_addr = self.addr_at(right, offset);
                    let left_elem = self.load_value_of_type(left_addr, element)?;
                    let right_elem = self.load_value_of_type(right_addr, element)?;
                    let elem_eq =
                        self.emit_structural_equality(left_elem, right_elem, element)?;
                    acc = self.func.builder.ins().band(acc, elem_eq);
                }
                acc
            }
            Type::Named(name) => self.emit_named_equality(left, right, name)?,
            _ => self.func.builder.ins().icmp(IntCC::Equal, left, right),
        })
    }

    fn emit_named_equality(
        &mut self,
        left: ir::Value,
        right: ir::Value,
        name: &str,
    ) -> Result<ir::Value, CraneliftError> {
        let ptr_size = self.module.pointer_type.bytes();
        let one = self.func.builder.ins().iconst(types::I8, 1);
        match crate::layout::resolve_type_item(name, self.module.types, &mut Vec::new()) {
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
                let mut acc = one;
                for (field_name, layout) in &field_layouts {
                    let field_type = s
                        .fields
                        .iter()
                        .find(|f| f.name == *field_name)
                        .map(|f| &f.type_)
                        .expect("field layout refers to a declared field");
                    let left_addr = self.addr_at(left, layout.offset);
                    let right_addr = self.addr_at(right, layout.offset);
                    let left_field = self.load_value_of_type(left_addr, field_type)?;
                    let right_field = self.load_value_of_type(right_addr, field_type)?;
                    let field_eq =
                        self.emit_structural_equality(left_field, right_field, field_type)?;
                    acc = self.func.builder.ins().band(acc, field_eq);
                }
                Ok(acc)
            }
            Some(HirItemKind::TupleStruct(t)) => {
                let mut acc = one;
                for (index, element) in t.types.iter().enumerate() {
                    let offset =
                        crate::layout::tuple_field_offset(index, &t.types, self.module.types, ptr_size);
                    let left_addr = self.addr_at(left, offset);
                    let right_addr = self.addr_at(right, offset);
                    let left_elem = self.load_value_of_type(left_addr, element)?;
                    let right_elem = self.load_value_of_type(right_addr, element)?;
                    let elem_eq =
                        self.emit_structural_equality(left_elem, right_elem, element)?;
                    acc = self.func.builder.ins().band(acc, elem_eq);
                }
                Ok(acc)
            }
            Some(HirItemKind::Enum(e)) => {
                let all_variant_data: Vec<Vec<Type>> = e
                    .variants
                    .iter()
                    .map(|variant| match &variant.data {
                        Some(HirEnumVariantData::Tuple(types)) => types.clone(),
                        Some(HirEnumVariantData::Struct(fields)) => {
                            fields.iter().map(|f| f.type_.clone()).collect()
                        }
                        None => Vec::new(),
                    })
                    .collect();
                let (_, data_offset, _) =
                    crate::layout::enum_layout(&all_variant_data, self.module.types, ptr_size);
                let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
                let left_disc = self.func.builder.ins().load(types::I8, mflags, left, 0);
                let right_disc = self.func.builder.ins().load(types::I8, mflags, right, 0);
                let mut acc = self
                    .func
                    .builder
                    .ins()
                    .icmp(IntCC::Equal, left_disc, right_disc);
                for (index, variant) in e.variants.iter().enumerate() {
                    let payload_types: Vec<Type> = match &variant.data {
                        Some(HirEnumVariantData::Tuple(types)) => types.clone(),
                        Some(HirEnumVariantData::Struct(fields)) => {
                            fields.iter().map(|f| f.type_.clone()).collect()
                        }
                        None => Vec::new(),
                    };
                    if payload_types.is_empty() {
                        continue;
                    }
                    let variant_const =
                        self.func.builder.ins().iconst(types::I8, index as i64);
                    let cond = self
                        .func
                        .builder
                        .ins()
                        .icmp(IntCC::Equal, left_disc, variant_const);
                    let mut fields_eq = one;
                    let mut offset_acc = 0u32;
                    for field_type in payload_types.iter() {
                        let field_align =
                            crate::layout::align_of(field_type, self.module.types, ptr_size);
                        offset_acc = crate::layout::align_up(offset_acc, field_align);
                        let left_addr = self.addr_at(left, data_offset + offset_acc);
                        let right_addr = self.addr_at(right, data_offset + offset_acc);
                        let left_field =
                            self.load_value_of_type(left_addr, field_type)?;
                        let right_field =
                            self.load_value_of_type(right_addr, field_type)?;
                        let field_eq =
                            self.emit_structural_equality(left_field, right_field, field_type)?;
                        fields_eq = self.func.builder.ins().band(fields_eq, field_eq);
                        offset_acc +=
                            crate::layout::size_of(field_type, self.module.types, ptr_size);
                    }
                    let acc_with_fields = self.func.builder.ins().band(acc, fields_eq);
                    acc = self.func.builder.ins().select(cond, acc_with_fields, acc);
                }
                Ok(acc)
            }
            Some(HirItemKind::TypeAlias(alias)) => {
                self.emit_structural_equality(left, right, &alias.type_)
            }
            _ => Err(CraneliftError::Msg(format!(
                "equality not supported for type `{name}`"
            ))),
        }
    }

    pub(super) fn emit_memcmp_diff(
        &mut self,
        left: ir::Value,
        right: ir::Value,
        size: u32,
    ) -> ir::Value {
        let pointer_type = self.module.pointer_type;
        if size == 0 {
            return self.func.builder.ins().iconst(types::I8, 0);
        }
        let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
        let zero = self.func.builder.ins().iconst(types::I8, 0);
        let mut diff = zero;
        for byte_offset in 0..size {
            let off_val = self
                .func
                .builder
                .ins()
                .iconst(pointer_type, byte_offset as i64);
            let left_addr = self.func.builder.ins().iadd(left, off_val);
            let right_addr = self.func.builder.ins().iadd(right, off_val);
            let left_byte = self
                .func
                .builder
                .ins()
                .load(types::I8, mflags, left_addr, 0);
            let right_byte = self
                .func
                .builder
                .ins()
                .load(types::I8, mflags, right_addr, 0);
            let xored = self.func.builder.ins().bxor(left_byte, right_byte);
            diff = self.func.builder.ins().bor(diff, xored);
        }
        diff
    }

    fn compile_expr_match(
        &mut self,
        value: &HirExpression,
        arms: &[HirMatchArm],
        result_type: &Type,
    ) -> Result<ir::Value, CraneliftError> {
        let ptr_size = self.module.pointer_type.bytes();
        let result_slot = if matches!(result_type, Type::Primitive(Primitive::Unit)) {
            None
        } else {
            let slot = self
                .func
                .builder
                .create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    crate::layout::aggregate_slot_size(result_type, self.module.types, ptr_size),
                    0,
                ));
            let ptr = self
                .func
                .builder
                .ins()
                .stack_addr(self.module.pointer_type, slot, 0);
            Some((result_type.clone(), ptr))
        };

        let header = self.func.builder.create_block();
        let merge = self.func.builder.create_block();
        let after = self.func.builder.create_block();
        let check_blocks: Vec<ir::Block> = (0..arms.len())
            .map(|_| self.func.builder.create_block())
            .collect();
        let arm_blocks: Vec<ir::Block> = (0..arms.len())
            .map(|_| self.func.builder.create_block())
            .collect();
        let body_blocks: Vec<ir::Block> = (0..arms.len())
            .map(|_| self.func.builder.create_block())
            .collect();

        self.func.builder.ins().jump(header, &[]);
        self.func.builder.switch_to_block(header);
        let scrutinee_val = self.compile_expr(value)?;
        let base = self.materialize_scrutinee(scrutinee_val, &value.type_)?;
        let first = check_blocks.first().copied().unwrap_or(merge);
        self.func.builder.ins().jump(first, &[]);
        self.func.builder.seal_block(header);

        for (index, arm) in arms.iter().enumerate() {
            self.func.builder.switch_to_block(check_blocks[index]);
            if index == arms.len() - 1 {
                self.func.builder.ins().jump(arm_blocks[index], &[]);
            } else {
                let cond = self.compile_pattern_condition(&arm.pattern, base)?;
                self.func.builder.ins().brif(
                    cond,
                    arm_blocks[index],
                    &[],
                    check_blocks[index + 1],
                    &[],
                );
            }
        }

        for (index, arm) in arms.iter().enumerate() {
            self.func.builder.switch_to_block(arm_blocks[index]);
            self.bind_pattern_vars(&arm.pattern, base)?;
            if let Some(guard) = &arm.guard {
                let guard_val = self.compile_expr(guard)?;
                if index == arms.len() - 1 {
                    // ponytail: last arm never has a failing guard (exhaustiveness
                    // requires an earlier non-guarded catch-all), trap as fallback.
                    let trap_block = self.func.builder.create_block();
                    self.func
                        .builder
                        .ins()
                        .brif(guard_val, body_blocks[index], &[], trap_block, &[]);
                    self.func.builder.switch_to_block(trap_block);
                    self.func
                        .builder
                        .ins()
                        .trap(ir::TrapCode::user(1).expect("valid user trap code"));
                    self.func.builder.seal_block(trap_block);
                    self.func.builder.switch_to_block(arm_blocks[index]);
                } else {
                    self.func
                        .builder
                        .ins()
                        .brif(guard_val, body_blocks[index], &[], check_blocks[index + 1], &[]);
                }
            } else {
                self.func.builder.ins().jump(body_blocks[index], &[]);
            }
            self.func.builder.seal_block(arm_blocks[index]);
        }

        for block in &check_blocks {
            self.func.builder.seal_block(*block);
        }

        for (index, arm) in arms.iter().enumerate() {
            self.func.builder.switch_to_block(body_blocks[index]);
            self.compile_match_body(&arm.body, &result_slot, merge)?;
            self.func.builder.seal_block(body_blocks[index]);
        }

        self.func.builder.switch_to_block(merge);
        self.func.builder.seal_block(merge);

        let result_val = if let Some((res_type, res_ptr)) = &result_slot {
            self.load_match_result(res_type, *res_ptr)?
        } else {
            self.func.builder.ins().iconst(types::I8, 0)
        };

        self.func.builder.ins().jump(after, &[]);
        self.func.builder.switch_to_block(after);
        self.func.builder.seal_block(after);
        Ok(result_val)
    }

    fn compile_match_body(
        &mut self,
        stmts: &[HirStatement],
        result_slot: &Option<(Type, ir::Value)>,
        merge: ir::Block,
    ) -> Result<(), CraneliftError> {
        let mut terminated = false;
        for (index, stmt) in stmts.iter().enumerate() {
            if terminated {
                break;
            }
            if index == stmts.len() - 1
                && let HirStatement {
                    kind: HirStatementKind::Value(val_expr, _),
                    ..
                } = stmt
            {
                let val = self.compile_expr(val_expr)?;
                if let Some((res_type, res_ptr)) = result_slot {
                    self.store_by_value(res_type, val, *res_ptr)?;
                }
                self.func.builder.ins().jump(merge, &[]);
                terminated = true;
                break;
            }
            self.compile_stmt(stmt, &mut terminated)?;
        }
        if !terminated {
            self.func.builder.ins().jump(merge, &[]);
        }
        Ok(())
    }

    fn load_match_result(
        &mut self,
        res_type: &Type,
        res_ptr: ir::Value,
    ) -> Result<ir::Value, CraneliftError> {
        let ptr_size = self.module.pointer_type.bytes();
        let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
        if is_large_aggregate(res_type, self.module.types, ptr_size) {
            Ok(res_ptr)
        } else if crate::layout::is_aggregate(res_type) {
            Ok(self.func.builder.ins().load(types::I64, mflags, res_ptr, 0))
        } else {
            let clif_ty = ir_type_from_primitive(res_type, self.module.pointer_type);
            Ok(self.func.builder.ins().load(clif_ty, mflags, res_ptr, 0))
        }
    }

    /// Turn the scrutinee value into an address of its memory representation so
    /// patterns can read the discriminant and payload fields uniformly.
    fn materialize_scrutinee(
        &mut self,
        val: ir::Value,
        ty: &Type,
    ) -> Result<ir::Value, CraneliftError> {
        let ptr_size = self.module.pointer_type.bytes();
        if is_large_aggregate(ty, self.module.types, ptr_size) {
            return Ok(val);
        }
        let slot = self
            .func
            .builder
            .create_sized_stack_slot(StackSlotData::new(
                StackSlotKind::ExplicitSlot,
                crate::layout::aggregate_slot_size(ty, self.module.types, ptr_size),
                0,
            ));
        let base = self
            .func
            .builder
            .ins()
            .stack_addr(self.module.pointer_type, slot, 0);
        let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
        self.func.builder.ins().store(mflags, val, base, 0);
        Ok(base)
    }

    fn enum_layout_info(
        &self,
        ty: &Type,
    ) -> Result<(u32, u32, Vec<Vec<Type>>), CraneliftError> {
        let Type::Named(name) = ty else {
            return Err(CraneliftError::Msg(
                "expected enum type in pattern".to_string(),
            ));
        };
        match crate::layout::resolve_type_item(name, self.module.types, &mut Vec::new()) {
            Some(HirItemKind::Enum(e)) => {
                let all_variant_data: Vec<Vec<Type>> = e
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
                let (_, data_offset, disc_size) = crate::layout::enum_layout(
                    &all_variant_data,
                    self.module.types,
                    self.module.pointer_type.bytes(),
                );
                Ok((data_offset, disc_size, all_variant_data))
            }
            _ => Err(CraneliftError::Msg(format!(
                "type `{name}` is not an enum"
            ))),
        }
    }

    fn addr_at(&mut self, addr: ir::Value, offset: u32) -> ir::Value {
        if offset == 0 {
            addr
        } else {
            let off_val = self
                .func
                .builder
                .ins()
                .iconst(self.module.pointer_type, offset as i64);
            self.func.builder.ins().iadd(addr, off_val)
        }
    }

    /// Addresses of the sub-patterns' values relative to `addr`, using the same
    /// layout rules as literal construction (enum payload, struct fields).
    fn pattern_sub_children<'p>(
        &mut self,
        pattern: &'p HirPattern,
        addr: ir::Value,
    ) -> Result<Vec<(ir::Value, &'p HirPattern)>, CraneliftError> {
        let ptr_size = self.module.pointer_type.bytes();
        match &pattern.kind {
            HirPatternKind::EnumVariant {
                variant_index,
                patterns,
                ..
            } => {
                let (data_offset, _, all_variant_data) = self.enum_layout_info(&pattern.type_)?;
                let payload_types = &all_variant_data[*variant_index];
                let mut children = Vec::new();
                let mut acc = 0u32;
                for (index, sub) in patterns.iter().enumerate() {
                    let elem_align = crate::layout::align_of(
                        &payload_types[index],
                        self.module.types,
                        ptr_size,
                    );
                    acc = crate::layout::align_up(acc, elem_align);
                    children.push((self.addr_at(addr, data_offset + acc), sub));
                    acc +=
                        crate::layout::size_of(&payload_types[index], self.module.types, ptr_size);
                }
                Ok(children)
            }
            HirPatternKind::Struct { fields, .. } => {
                let Type::Named(type_name) = &pattern.type_ else {
                    return Err(CraneliftError::Msg(
                        "expected named type in struct pattern".to_string(),
                    ));
                };
                match crate::layout::resolve_type_item(type_name, self.module.types, &mut Vec::new())
                {
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
                        let mut children = Vec::new();
                        for (name, sub) in fields {
                            let field_idx = s
                                .fields
                                .iter()
                                .position(|f| f.name == *name)
                                .ok_or_else(|| {
                                    CraneliftError::Msg(format!(
                                        "struct `{type_name}` has no field `{name}`"
                                    ))
                                })?;
                            children.push((
                                self.addr_at(addr, field_layouts[field_idx].1.offset),
                                sub,
                            ));
                        }
                        Ok(children)
                    }
                    Some(HirItemKind::TupleStruct(t)) => {
                        let mut children = Vec::new();
                        for (index, (_, sub)) in fields.iter().enumerate() {
                            let offset = crate::layout::tuple_field_offset(
                                index,
                                &t.types,
                                self.module.types,
                                ptr_size,
                            );
                            children.push((self.addr_at(addr, offset), sub));
                        }
                        Ok(children)
                    }
                    _ => Err(CraneliftError::Msg(format!(
                        "type `{type_name}` is not a struct"
                    ))),
                }
            }
            HirPatternKind::Tuple { elements, .. } => {
                let Type::Tuple(element_types) = &pattern.type_ else {
                    return Err(CraneliftError::Msg(
                        "expected tuple type in tuple pattern".to_string(),
                    ));
                };
                let mut children = Vec::new();
                for (index, sub) in elements.iter().enumerate() {
                    let offset = crate::layout::tuple_field_offset(
                        index,
                        element_types,
                        self.module.types,
                        ptr_size,
                    );
                    children.push((self.addr_at(addr, offset), sub));
                }
                Ok(children)
            }
            _ => Ok(Vec::new()),
        }
    }

    fn compile_pattern_condition(
        &mut self,
        pattern: &HirPattern,
        addr: ir::Value,
    ) -> Result<ir::Value, CraneliftError> {
        let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
        match &pattern.kind {
            HirPatternKind::EnumVariant {
                variant_index, ..
            } => {
                let (_, disc_size, _) = self.enum_layout_info(&pattern.type_)?;
                let disc_ty = if disc_size == 2 {
                    types::I16
                } else {
                    types::I8
                };
                let disc = self.func.builder.ins().load(disc_ty, mflags, addr, 0);
                let target = self
                    .func
                    .builder
                    .ins()
                    .iconst(disc_ty, *variant_index as i64);
                let mut cond = self
                    .func
                    .builder
                    .ins()
                    .icmp(IntCC::Equal, disc, target);
                let children = self.pattern_sub_children(pattern, addr)?;
                for (sub_addr, sub) in children {
                    let sub_cond = self.compile_pattern_condition(sub, sub_addr)?;
                    cond = self.func.builder.ins().band(cond, sub_cond);
                }
                Ok(cond)
            }
            HirPatternKind::Literal { value, .. } => {
                let clif_ty = ir_type_from_primitive(&pattern.type_, self.module.pointer_type);
                let loaded = self.func.builder.ins().load(clif_ty, mflags, addr, 0);
                let constant = match value {
                    LiteralValue::Int(v) => self.emit_iconst(clif_ty, *v),
                    LiteralValue::Bool(b) => self.func.builder.ins().iconst(types::I8, *b as i64),
                    LiteralValue::Char(c) => self.func.builder.ins().iconst(types::I32, *c as i64),
                    LiteralValue::String(_) => {
                        return Err(CraneliftError::Msg(
                            "string patterns not supported in codegen".to_string(),
                        ));
                    }
                };
                if clif_ty == types::F32 || clif_ty == types::F64 {
                    Ok(self
                        .func
                        .builder
                        .ins()
                        .fcmp(FloatCC::Equal, loaded, constant))
                } else {
                    Ok(self
                        .func
                        .builder
                        .ins()
                        .icmp(IntCC::Equal, loaded, constant))
                }
            }
            HirPatternKind::Struct { .. } | HirPatternKind::Tuple { .. } => {
                let mut cond = self.func.builder.ins().iconst(types::I8, 1);
                let children = self.pattern_sub_children(pattern, addr)?;
                for (sub_addr, sub) in children {
                    let sub_cond = self.compile_pattern_condition(sub, sub_addr)?;
                    cond = self.func.builder.ins().band(cond, sub_cond);
                }
                Ok(cond)
            }
            HirPatternKind::Wildcard(_) | HirPatternKind::Ident { .. } => {
                Ok(self.func.builder.ins().iconst(types::I8, 1))
            }
        }
    }

    fn bind_pattern_vars(
        &mut self,
        pattern: &HirPattern,
        addr: ir::Value,
    ) -> Result<(), CraneliftError> {
        match &pattern.kind {
            HirPatternKind::Ident { name, .. } => {
                let val = self.load_pattern_value(pattern, addr)?;
                self.bind_pattern_var(name, val, &pattern.type_)
            }
            HirPatternKind::Wildcard(_) | HirPatternKind::Literal { .. } => Ok(()),
            _ => {
                let children = self.pattern_sub_children(pattern, addr)?;
                for (sub_addr, sub) in children {
                    self.bind_pattern_vars(sub, sub_addr)?;
                }
                Ok(())
            }
        }
    }

    fn load_pattern_value(
        &mut self,
        pattern: &HirPattern,
        addr: ir::Value,
    ) -> Result<ir::Value, CraneliftError> {
        let ptr_size = self.module.pointer_type.bytes();
        if is_large_aggregate(&pattern.type_, self.module.types, ptr_size) {
            return Ok(addr);
        }
        let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
        if crate::layout::is_aggregate(&pattern.type_) {
            Ok(self.func.builder.ins().load(types::I64, mflags, addr, 0))
        } else {
            let clif_ty = ir_type_from_primitive(&pattern.type_, self.module.pointer_type);
            Ok(self.func.builder.ins().load(clif_ty, mflags, addr, 0))
        }
    }

    fn bind_pattern_var(
        &mut self,
        name: &str,
        val: ir::Value,
        ty: &Type,
    ) -> Result<(), CraneliftError> {
        let ptr_size = self.module.pointer_type.bytes();
        if is_large_aggregate(ty, self.module.types, ptr_size) {
            let slot = self
                .func
                .builder
                .create_sized_stack_slot(StackSlotData::new(
                    StackSlotKind::ExplicitSlot,
                    crate::layout::aggregate_slot_size(ty, self.module.types, ptr_size),
                    0,
                ));
            let dest = self
                .func
                .builder
                .ins()
                .stack_addr(self.module.pointer_type, slot, 0);
            self.emit_memcpy(
                dest,
                val,
                crate::layout::aggregate_copy_size(ty, self.module.types, ptr_size),
            )?;
            self.func.vars.insert(
                name.to_string(),
                VarInfo {
                    slot: VarSlot::StackSlot(slot, self.module.pointer_type),
                    vinyl_type: ty.clone(),
                },
            );
            Ok(())
        } else {
            let clif_type = ir_type_from_primitive(ty, self.module.pointer_type);
            let mode = var_mode(name, false, self.func.ref_vars);
            let (slot, _) = build_var_info(
                self.func.builder,
                ty,
                clif_type,
                val,
                mode,
                self.module.pointer_type,
            );
            self.func.vars.insert(
                name.to_string(),
                VarInfo {
                    slot,
                    vinyl_type: ty.clone(),
                },
            );
            Ok(())
        }
    }
}
