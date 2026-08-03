use cranelift_codegen::ir::{self, InstBuilder, StackSlotData, StackSlotKind, types};

use vinyl_parser::ast::types::Primitive;
use vinyl_typecheck::hir::AssignOp;
use vinyl_typecheck::hir::{
    HirAssignTarget, HirExpression, HirExpressionKind, HirItemKind, HirStatement, HirStatementKind,
    Type,
};

use super::state::{CodegenCtx, VarInfo, VarSlot};
use super::types::{
    element_byte_size, extract_array_element_type, ir_type_from_primitive, is_large_aggregate,
};
use super::variable::{build_var_info, var_mode};
use crate::CraneliftError;

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
                let ptr_size = self.module.pointer_type.bytes();
                if is_large_aggregate(type_, self.module.types, ptr_size) {
                    // todo: always stack-backed, keep small aggregates in SSA values for perf
                    let val = self.compile_expr(value)?;
                    let total_size = crate::layout::size_of(type_, self.module.types, ptr_size);
                    let slot = self
                        .func
                        .builder
                        .create_sized_stack_slot(StackSlotData::new(
                            StackSlotKind::ExplicitSlot,
                            total_size,
                            0,
                        ));
                    let dest =
                        self.func
                            .builder
                            .ins()
                            .stack_addr(self.module.pointer_type, slot, 0);
                    self.emit_memcpy(dest, val, total_size)?;
                    self.func.vars.insert(
                        name.clone(),
                        VarInfo {
                            slot: VarSlot::StackSlot(slot, self.module.pointer_type),
                            vinyl_type: type_.clone(),
                        },
                    );
                } else {
                    let clif_type = ir_type_from_primitive(type_, self.module.pointer_type);
                    let val = self.compile_expr(value)?;
                    let mode = var_mode(name, *mutable, self.func.ref_vars);
                    let (slot, _) = build_var_info(
                        self.func.builder,
                        type_,
                        clif_type,
                        val,
                        mode,
                        self.module.pointer_type,
                    );
                    self.func.vars.insert(
                        name.clone(),
                        VarInfo {
                            slot,
                            vinyl_type: type_.clone(),
                        },
                    );
                }
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
                        self.func.builder.ins().return_(&[val]);
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
                if matches!(expr.type_, Type::Primitive(Primitive::Unit)) {
                    self.func.builder.ins().return_(&[]);
                } else {
                    self.func.builder.ins().return_(&[val]);
                }
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
            self.apply_compound_op(current, value, op, value_expr)?
        } else {
            value
        };

        match target {
            HirAssignTarget::Ident(name, _) => self.write_var(name, write_val),
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
                self.func.builder.ins().store(mflags, write_val, addr, 0);
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
                    self.func.builder.ins().store(mflags, write_val, addr, 0);
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
        let elem_size = element_byte_size(elem_type, self.module.pointer_type);
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
            Type::Named(type_name) => match self.module.types.get(type_name) {
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

    fn apply_compound_op(
        &mut self,
        current: ir::Value,
        value: ir::Value,
        op: &AssignOp,
        value_expr: &HirExpression,
    ) -> Result<ir::Value, CraneliftError> {
        Ok(match op {
            AssignOp::Eq => value,
            AssignOp::AddEq => self.func.builder.ins().iadd(current, value),
            AssignOp::SubEq => self.func.builder.ins().isub(current, value),
            AssignOp::MulEq => self.func.builder.ins().imul(current, value),
            AssignOp::DivEq => self.func.builder.ins().sdiv(current, value),
            AssignOp::RemEq => self.func.builder.ins().srem(current, value),
            AssignOp::BitAndEq => self.func.builder.ins().band(current, value),
            AssignOp::BitOrEq => self.func.builder.ins().bor(current, value),
            AssignOp::BitXorEq => self.func.builder.ins().bxor(current, value),
            AssignOp::ShlEq => self.func.builder.ins().ishl(current, value),
            AssignOp::ShrEq => self.func.builder.ins().sshr(current, value),
            AssignOp::PowEq => self.compile_pow(current, value, value_expr)?,
        })
    }
}
