use cranelift_codegen::ir::{self, InstBuilder, StackSlotData, StackSlotKind, types};

use vinyl_parser::ast::types::Primitive;
use vinyl_typecheck::hir::AssignOp;
use vinyl_typecheck::hir::{
    HirAssignTarget, HirExpression, HirExpressionKind, HirItemKind, HirStatement, HirStatementKind,
    Type,
};

use super::state::{CodegenCtx, VarInfo, VarSlot};
use super::types::{extract_array_element_type, ir_type_from_primitive, is_large_aggregate};
use crate::CraneliftError;
use crate::locals::LocalBuilder;

impl<'a> CodegenCtx<'a> {
    pub fn compile_stmt(
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
                ..
            } => {
                let val = self.compile_expr(value)?;
                let address_taken = self.func.ref_vars.contains(name);
                let slot = LocalBuilder::new(self, name.clone())
                    .typed(type_.clone())
                    .initialized(val)
                    .mutable_if(*mutable)
                    .address_taken_if(address_taken)
                    .build()?;
                self.func.vars.insert(
                    name.clone(),
                    VarInfo {
                        slot,
                        vinyl_type: type_.clone(),
                    },
                );
                Ok(())
            }
            HirStatementKind::Expr(expr, _) => {
                self.compile_expr(expr)?;
                Ok(())
            }
            HirStatementKind::Return(expr, _) => {
                match expr {
                    Some(e) => {
                        let val = self.compile_expr(e)?;
                        self.emit_return(val)?;
                    }
                    None => {
                        self.func.builder.ins().return_(&[]);
                    }
                }
                *terminated = true;
                Ok(())
            }
            HirStatementKind::Value(expr, _) => {
                let val = self.compile_expr(expr)?;
                self.emit_return(val)?;
                *terminated = true;
                Ok(())
            }
            HirStatementKind::Loop { body, .. } => {
                let saved_break = self.func.break_target;
                let saved_continue = self.func.continue_target;

                let header = self.func.builder.create_block();
                let exit = self.func.builder.create_block();

                self.func.builder.ins().jump(header, &[]);
                self.func.builder.switch_to_block(header);

                self.func.break_target = Some(exit);
                self.func.continue_target = Some(header);

                let mut body_terminated = false;
                for stmt in body {
                    if let HirStatement {
                        kind: HirStatementKind::Value(expr, _),
                        ..
                    } = stmt
                    {
                        self.compile_expr(expr)?;
                    } else {
                        self.compile_stmt(stmt, &mut body_terminated)?;
                    }
                }
                if !body_terminated {
                    self.func.builder.ins().jump(header, &[]);
                }

                self.func.builder.seal_block(header);
                self.func.builder.switch_to_block(exit);
                self.func.builder.seal_block(exit);

                self.func.break_target = saved_break;
                self.func.continue_target = saved_continue;

                *terminated = false;
                Ok(())
            }
            HirStatementKind::Break(_) => {
                match self.func.break_target {
                    Some(target) => {
                        self.func.builder.ins().jump(target, &[]);
                    }
                    None => {
                        return Err(CraneliftError::Msg("break outside loop".to_string()));
                    }
                }
                *terminated = true;
                Ok(())
            }
            HirStatementKind::Continue(_) => {
                match self.func.continue_target {
                    Some(target) => {
                        self.func.builder.ins().jump(target, &[]);
                    }
                    None => {
                        return Err(CraneliftError::Msg("continue outside loop".to_string()));
                    }
                }
                *terminated = true;
                Ok(())
            }
            HirStatementKind::Assign {
                target, op, value, ..
            } => {
                let val = self.compile_expr(value)?;
                self.compile_assign_target(target, op, val, value)?;
                Ok(())
            }
        }
    }

    fn emit_return(&mut self, val: ir::Value) -> Result<(), CraneliftError> {
        let return_type = &self.func.return_type;
        if crate::layout::is_aggregate(return_type) {
            let ptr_size = self.module.pointer_type.bytes();
            let chunks =
                crate::layout::aggregate_register_count(return_type, self.module.types, ptr_size);
            match chunks {
                0 => {
                    // >16 bytes: copy into the caller-provided sret pointer
                    let sret_ptr = self
                        .func
                        .sret_ptr
                        .ok_or_else(|| CraneliftError::Msg("missing sret pointer".to_string()))?;
                    self.emit_memcpy(
                        sret_ptr,
                        val,
                        crate::layout::size_of(return_type, self.module.types, ptr_size),
                    )?;
                    self.func.builder.ins().return_(&[]);
                }
                2 => {
                    // 9-16 bytes: two 64-bit register chunks, stored into the 16-byte slot
                    let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
                    let chunk0 = self.func.builder.ins().load(types::I64, mflags, val, 0);
                    let addr1 = {
                        let off = self.func.builder.ins().iconst(self.module.pointer_type, 8);
                        self.func.builder.ins().iadd(val, off)
                    };
                    let chunk1 = self.func.builder.ins().load(types::I64, mflags, addr1, 0);
                    self.func.builder.ins().return_(&[chunk0, chunk1]);
                }
                _ => {
                    // <=8 bytes: passed as a single 64-bit register value
                    self.func.builder.ins().return_(&[val]);
                }
            }
        } else if matches!(return_type, Type::Primitive(Primitive::Unit)) {
            self.func.builder.ins().return_(&[]);
        } else {
            self.func.builder.ins().return_(&[val]);
        }
        Ok(())
    }

    fn compile_assign_target(
        &mut self,
        target: &HirAssignTarget,
        op: &AssignOp,
        value: ir::Value,
        value_expr: &HirExpression,
    ) -> Result<(), CraneliftError> {
        let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
        let is_compound = *op != AssignOp::Eq;

        let write_val = if is_compound {
            let current = match target {
                HirAssignTarget::Ident(name, _) => self.read_var(name)?,
                HirAssignTarget::Deref(inner, _) => {
                    let ptr = match &inner.kind {
                        HirExpressionKind::Ident(name, _) => self.read_var_raw(name)?,
                        _ => self.compile_expr(inner)?,
                    };
                    let ty = self.func.builder.func.dfg.value_type(value);
                    self.func.builder.ins().load(ty, mflags, ptr, 0)
                }
                HirAssignTarget::Index { array, index, .. } => {
                    let array_ptr = self.compile_expr(array)?;
                    let index_val = self.compile_expr(index)?;
                    let addr = self.compute_index_addr(array_ptr, index_val, target)?;
                    let ty = self.func.builder.func.dfg.value_type(value);
                    self.func.builder.ins().load(ty, mflags, addr, 0)
                }
                HirAssignTarget::Field { object, name, .. } => {
                    let val = self.compile_expr(object.as_ref())?;
                    let ptr_size = self.module.pointer_type.bytes();
                    let offset = self.resolve_field_offset(&object.type_, name, ptr_size)?;
                    let mflags = cranelift_codegen::ir::MachMemFlags::trusted();
                    let obj_is_ptr = is_large_aggregate(&object.type_, self.module.types, ptr_size);
                    if obj_is_ptr {
                        let addr = if offset == 0 {
                            val
                        } else {
                            let off_val = self
                                .func
                                .builder
                                .ins()
                                .iconst(self.module.pointer_type, offset as i64);
                            self.func.builder.ins().iadd(val, off_val)
                        };
                        let field_ty = self.resolve_field_type(&object.type_, name, ptr_size)?;
                        let clif_ty = ir_type_from_primitive(&field_ty, self.module.pointer_type);
                        self.func.builder.ins().load(clif_ty, mflags, addr, 0)
                    } else {
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
                        self.func.builder.ins().store(mflags, val, base, 0);
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
                        let field_ty = self.resolve_field_type(&object.type_, name, ptr_size)?;
                        let clif_ty = ir_type_from_primitive(&field_ty, self.module.pointer_type);
                        self.func.builder.ins().load(clif_ty, mflags, addr, 0)
                    }
                }
            };
            self.apply_compound_op(current, value, op, value_expr, target)?
        } else {
            value
        };

        match target {
            HirAssignTarget::Ident(name, _) => {
                let ptr_size = self.module.pointer_type.bytes();
                let needs_copy = self
                    .func
                    .vars
                    .get(name.as_str())
                    .map(|info| is_large_aggregate(&info.vinyl_type, self.module.types, ptr_size))
                    .unwrap_or(false);
                if needs_copy {
                    // Large aggregates are stack-backed and the RHS is an
                    // address; copy the whole value instead of storing a pointer.
                    let slot = match self.func.vars.get(name.as_str()) {
                        Some(VarInfo {
                            slot: VarSlot::StackSlot(slot, _),
                            ..
                        }) => *slot,
                        _ => {
                            return Err(CraneliftError::Msg(format!(
                                "aggregate variable `{name}` is not stack-backed"
                            )));
                        }
                    };
                    let ty = self.func.vars[name.as_str()].vinyl_type.clone();
                    let addr =
                        self.func
                            .builder
                            .ins()
                            .stack_addr(self.module.pointer_type, slot, 0);
                    self.store_by_value(&ty, write_val, addr)
                } else {
                    self.write_var(name, write_val)
                }
            }
            HirAssignTarget::Deref(inner, _) => {
                let ptr = match &inner.kind {
                    HirExpressionKind::Ident(name, _) => self.read_var_raw(name)?,
                    _ => self.compile_expr(inner)?,
                };
                self.func.builder.ins().store(mflags, write_val, ptr, 0);
                Ok(())
            }
            HirAssignTarget::Index { array, index, .. } => {
                let array_ptr = self.compile_expr(array)?;
                let index_val = self.compile_expr(index)?;
                let addr = self.compute_index_addr(array_ptr, index_val, target)?;
                let elem_type = match extract_array_element_type(target) {
                    Some(t) => t,
                    None => &Type::Primitive(Primitive::Int32),
                };
                self.store_by_value(elem_type, write_val, addr)?;
                Ok(())
            }
            HirAssignTarget::Field { object, name, .. } => {
                let ptr_size = self.module.pointer_type.bytes();
                let var_name = match object.as_ref() {
                    HirExpression {
                        kind: HirExpressionKind::Ident(name, _),
                        ..
                    } => Some(name.clone()),
                    _ => None,
                };
                let obj = self.compile_expr(object.as_ref())?;
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
                                "cannot assign to field on type `{type_name}`"
                            )));
                        }
                    },
                    _ => {
                        return Err(CraneliftError::Msg(format!(
                            "field assignment not supported for type {:?}",
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
                    let field_type = self.resolve_field_type(&object.type_, name, ptr_size)?;
                    self.store_by_value(&field_type, write_val, addr)?;
                    Ok(())
                } else if let Some(var_name) = var_name {
                    // Small aggregate packed as i64: materialize on stack, modify field, store back
                    // todo: avoid temp stack slot, use bit manipulation
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
                    self.func.builder.ins().store(mflags, write_val, addr, 0);
                    let new_val = self.func.builder.ins().load(types::I64, mflags, base, 0);
                    self.write_var(&var_name, new_val)
                } else {
                    Err(CraneliftError::Msg(
                        "field assignment only supported on identifiers and large aggregates"
                            .to_string(),
                    ))
                }
            }
        }
    }

    fn compute_index_addr(
        &mut self,
        array_ptr: ir::Value,
        index_val: ir::Value,
        target: &HirAssignTarget,
    ) -> Result<ir::Value, CraneliftError> {
        let index_ty = self.func.builder.func.dfg.value_type(index_val);
        let index_wide = if index_ty != self.module.pointer_type {
            self.func
                .builder
                .ins()
                .uextend(self.module.pointer_type, index_val)
        } else {
            index_val
        };
        let elem_type = match extract_array_element_type(target) {
            Some(t) => t,
            None => &Type::Primitive(Primitive::Int32),
        };
        let elem_size = crate::layout::array_element_stride(
            elem_type,
            self.module.types,
            self.module.pointer_type.bytes(),
        );
        let size_val = self
            .func
            .builder
            .ins()
            .iconst(self.module.pointer_type, elem_size as i64);
        let offset = self.func.builder.ins().imul(index_wide, size_val);
        Ok(self.func.builder.ins().iadd(array_ptr, offset))
    }

    fn resolve_field_offset(
        &mut self,
        object_type: &Type,
        name: &str,
        ptr_size: u32,
    ) -> Result<u32, CraneliftError> {
        match object_type {
            Type::Tuple(element_types) => {
                let index: usize = name
                    .parse()
                    .map_err(|_| CraneliftError::Msg(format!("invalid tuple index `{name}`")))?;
                Ok(crate::layout::tuple_field_offset(
                    index,
                    element_types,
                    self.module.types,
                    ptr_size,
                ))
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
                    let field_idx =
                        s.fields
                            .iter()
                            .position(|f| f.name == *name)
                            .ok_or_else(|| {
                                CraneliftError::Msg(format!(
                                    "struct `{type_name}` has no field `{name}`"
                                ))
                            })?;
                    Ok(field_layouts[field_idx].1.offset)
                }
                Some(HirItemKind::TupleStruct(t)) => {
                    let index: usize = name.parse().map_err(|_| {
                        CraneliftError::Msg(format!("invalid tuple struct field `{name}`"))
                    })?;
                    Ok(crate::layout::tuple_field_offset(
                        index,
                        &t.types,
                        self.module.types,
                        ptr_size,
                    ))
                }
                _ => Err(CraneliftError::Msg(format!(
                    "cannot access field on type `{type_name}`"
                ))),
            },
            _ => Err(CraneliftError::Msg(
                "field access not supported".to_string(),
            )),
        }
    }

    fn resolve_field_type(
        &mut self,
        object_type: &Type,
        name: &str,
        _ptr_size: u32,
    ) -> Result<Type, CraneliftError> {
        match object_type {
            Type::Tuple(element_types) => {
                let index: usize = name
                    .parse()
                    .map_err(|_| CraneliftError::Msg(format!("invalid tuple index `{name}`")))?;
                Ok(element_types[index].clone())
            }
            Type::Named(type_name) => match crate::layout::resolve_type_item(
                type_name,
                self.module.types,
                &mut Vec::new(),
            ) {
                Some(HirItemKind::Struct(s)) => s
                    .fields
                    .iter()
                    .find(|f| f.name == *name)
                    .map(|f| f.type_.clone())
                    .ok_or_else(|| {
                        CraneliftError::Msg(format!("struct `{type_name}` has no field `{name}`"))
                    }),
                Some(HirItemKind::TupleStruct(t)) => {
                    let index: usize = name.parse().map_err(|_| {
                        CraneliftError::Msg(format!("invalid tuple struct field `{name}`"))
                    })?;
                    Ok(t.types[index].clone())
                }
                _ => Err(CraneliftError::Msg(format!(
                    "cannot access field on type `{type_name}`"
                ))),
            },
            _ => Err(CraneliftError::Msg(
                "field access not supported".to_string(),
            )),
        }
    }

    fn target_is_unsigned(&mut self, target: &HirAssignTarget) -> bool {
        let target_type: Option<Type> = match target {
            HirAssignTarget::Ident(name, _) => {
                self.func.vars.get(name).map(|v| v.vinyl_type.clone())
            }
            HirAssignTarget::Index { array, .. } => match &array.type_ {
                Type::Array { element, .. } => Some((**element).clone()),
                _ => None,
            },
            HirAssignTarget::Field { object, name, .. } => {
                let ptr_size = self.module.pointer_type.bytes();
                self.resolve_field_type(&object.type_, name, ptr_size).ok()
            }
            HirAssignTarget::Deref(inner, _) => match &inner.type_ {
                Type::Ref(inner_ty) => Some((**inner_ty).clone()),
                other => Some(other.clone()),
            },
        };
        matches!(
            target_type,
            Some(Type::Primitive(
                Primitive::UInt8
                    | Primitive::UInt16
                    | Primitive::UInt32
                    | Primitive::UInt64
                    | Primitive::UInt128
                    | Primitive::USize
                    | Primitive::UInt
            ))
        )
    }

    fn apply_compound_op(
        &mut self,
        current: ir::Value,
        value: ir::Value,
        op: &AssignOp,
        value_expr: &HirExpression,
        target: &HirAssignTarget,
    ) -> Result<ir::Value, CraneliftError> {
        let ty = self.func.builder.func.dfg.value_type(current);
        let unsigned = self.target_is_unsigned(target);
        Ok(match op {
            AssignOp::Eq => value,
            AssignOp::AddEq => self.func.builder.ins().iadd(current, value),
            AssignOp::SubEq => self.func.builder.ins().isub(current, value),
            AssignOp::MulEq => self.func.builder.ins().imul(current, value),
            AssignOp::DivEq => self.emit_div_rem(current, value, ty, !unsigned).0,
            AssignOp::RemEq => self.emit_div_rem(current, value, ty, !unsigned).1,
            AssignOp::BitAndEq => self.func.builder.ins().band(current, value),
            AssignOp::BitOrEq => self.func.builder.ins().bor(current, value),
            AssignOp::BitXorEq => self.func.builder.ins().bxor(current, value),
            AssignOp::ShlEq => self.func.builder.ins().ishl(current, value),
            AssignOp::ShrEq => {
                if unsigned {
                    self.func.builder.ins().ushr(current, value)
                } else {
                    self.func.builder.ins().sshr(current, value)
                }
            }
            AssignOp::PowEq => self.compile_pow(current, value, value_expr)?,
        })
    }
}
