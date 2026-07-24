use cranelift_codegen::ir::{self, InstBuilder};

use vinyl_parser::ast::types::Primitive;
use vinyl_typecheck::hir::AssignOp;
use vinyl_typecheck::hir::{
    HirAssignTarget, HirExpressionKind, HirStatement, HirStatementKind, Type,
};

use super::state::CodegenCtx;
use super::types::{element_byte_size, extract_array_element_type, ir_type_from_primitive};
use super::variable::{build_var_info, var_mode};
use super::CraneliftError;

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
            } => {
                let clif_type = ir_type_from_primitive(type_, self.module.pointer_type);
                let val = self.compile_expr(value)?;
                let mode = var_mode(name, *mutable, self.func.ref_vars);
                let (slot, _) =
                    build_var_info(self.func.builder, type_, clif_type, val, mode, self.module.pointer_type);
                self.func.vars.insert(
                    name.clone(),
                    crate::cranelift::state::VarInfo {
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
                        self.func.builder.ins().return_(&[val]);
                    }
                    None => {
                        self.func.builder.ins().return_(&[]);
                    }
                }
                *terminated = true;
                Ok(())
            }
            HirStatementKind::Value(expr) => {
                let val = self.compile_expr(expr)?;
                if matches!(expr.type_, Type::Primitive(Primitive::Unit)) {
                    self.func.builder.ins().return_(&[]);
                } else {
                    self.func.builder.ins().return_(&[val]);
                }
                *terminated = true;
                Ok(())
            }
            HirStatementKind::Loop { body } => {
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
            HirStatementKind::Break => {
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
            HirStatementKind::Continue => {
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
            let current = match target {
                HirAssignTarget::Ident(name) => self.read_var(name)?,
                HirAssignTarget::Deref(inner) => {
                    let ptr = match &inner.kind {
                        HirExpressionKind::Ident(name) => self.read_var_raw(name)?,
                        _ => self.compile_expr(inner)?,
                    };
                    let ty = self.func.builder.func.dfg.value_type(value);
                    self.func.builder.ins().load(ty, mflags, ptr, 0)
                }
                HirAssignTarget::Index { array, index } => {
                    let array_ptr = self.compile_expr(array)?;
                    let index_val = self.compile_expr(index)?;
                    let addr = self.compute_index_addr(array_ptr, index_val, target)?;
                    let ty = self.func.builder.func.dfg.value_type(value);
                    self.func.builder.ins().load(ty, mflags, addr, 0)
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
                    HirExpressionKind::Ident(name) => self.read_var_raw(name)?,
                    _ => self.compile_expr(inner)?,
                };
                self.func.builder.ins().store(mflags, write_val, ptr, 0);
                Ok(())
            }
            HirAssignTarget::Index { array, index } => {
                let array_ptr = self.compile_expr(array)?;
                let index_val = self.compile_expr(index)?;
                let addr = self.compute_index_addr(array_ptr, index_val, target)?;
                self.func.builder.ins().store(mflags, write_val, addr, 0);
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
        let index_ty = self.func.builder.func.dfg.value_type(index_val);
        let index_wide = if index_ty != self.module.pointer_type {
            self.func.builder.ins().uextend(self.module.pointer_type, index_val)
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

    fn apply_compound_op(
        &mut self,
        current: ir::Value,
        value: ir::Value,
        op: &AssignOp,
    ) -> ir::Value {
        match op {
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
        }
    }
}
