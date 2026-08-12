use std::collections::HashMap;

use miette::SourceSpan;
use vinyl_parser::ast::expression::Expression;
use vinyl_parser::ast::item::FunctionDef;
use vinyl_parser::ast::operator::AssignOp;
use vinyl_parser::ast::statement::{AssignTarget, Statement};
use vinyl_parser::ast::types::Primitive;

use crate::error::{InferResult, TypeDiagnostic, TypeDiagnosticKind};
use crate::hir::{
    HirAssignTarget, HirExpression, HirExpressionKind, HirFunction, HirParam, HirStatement,
    HirStatementKind, Type,
};
use crate::infer::{InferState, TypeScheme};

impl InferState {
    pub(super) fn infer_function(
        &mut self,
        func: &FunctionDef,
        signatures: &HashMap<&str, &FunctionDef>,
    ) -> InferResult<HirFunction> {
        let mut params = Vec::new();
        for param in &func.params {
            let mutable = true;
            params.push(HirParam {
                span: param.span,
                name: param.name.clone(),
                mutable,
                type_: param.type_.clone(),
            });
            self.scope.bind(
                &param.name,
                TypeScheme {
                    type_: param.type_.clone(),
                    mutable,
                },
            );
        }

        let mut return_type = match &func.return_type {
            Some(t) => t.clone(),
            None => self.subs.fresh_var(),
        };

        if let Some(intrinsic) = crate::infer::intrinsic_kind(func) {
            return Ok(HirFunction {
                span: func.span,
                name: func.name.clone(),
                public: func.public,
                documentation: None,
                params,
                return_type,
                body: Vec::new(),
                intrinsic: Some(intrinsic),
            });
        }

        let resolved_ret = self.subs.apply(&return_type);
        if let Type::Ref(_) = &resolved_ret {
            self.errors.push(
                self.source
                    .error(func.span, TypeDiagnosticKind::CannotReturnRef),
            );
        }

        let prev_return = self.current_return_type.replace(return_type.clone());
        self.scope.push_scope();
        let body = self.infer_block(&func.body, signatures)?;
        self.scope.pop_scope();
        self.current_return_type = prev_return;

        if let Some(HirStatement {
            kind: HirStatementKind::Value(expr, _),
            ..
        }) = body.last()
        {
            let value_type = self.subs.apply(&expr.type_);
            let ret_type = self.subs.apply(&return_type);
            if let Err(e) = self
                .subs
                .unify(&self.source, &value_type, &ret_type, func.span)
            {
                self.errors.push(*e);
            }
        }

        if !body
            .last()
            .is_some_and(|s| matches!(s.kind, HirStatementKind::Value(_, _)))
        {
            if let Type::Var(id) = &return_type {
                self.subs.subs.remove(id);
            }
            return_type = Type::Primitive(Primitive::Unit);
        }
        let body = self.resolve_hir_stmts(body);
        return_type = self.resolve_hir_type(&return_type);

        self.errors.extend(self.collect_literal_type_errors(&body));

        Ok(HirFunction {
            span: func.span,
            name: func.name.clone(),
            public: func.public,
            documentation: None,
            params,
            return_type,
            body,
            intrinsic: None,
        })
    }

    pub(super) fn infer_block(
        &mut self,
        stmts: &[Statement],
        signatures: &HashMap<&str, &FunctionDef>,
    ) -> InferResult<Vec<HirStatement>> {
        let mut hir_stmts = Vec::new();
        let mut terminated = false;
        for stmt in stmts {
            if terminated {
                self.errors.push(
                    self.source
                        .error(stmt.span(), TypeDiagnosticKind::UnreachableStatement),
                );
            }
            hir_stmts.push(self.infer_stmt(stmt, signatures)?);
            match stmt {
                Statement::Return(..) | Statement::Break(..) | Statement::Continue(..) => {
                    terminated = true
                }
                _ => {}
            }
        }
        Ok(hir_stmts)
    }

    pub(super) fn infer_stmt(
        &mut self,
        stmt: &Statement,
        signatures: &HashMap<&str, &FunctionDef>,
    ) -> InferResult<HirStatement> {
        match stmt {
            Statement::Let {
                span,
                name,
                mutable,
                type_,
                value,
            } => {
                let hir_value = self.infer_expr(value, signatures)?;

                if let Some(ann) = type_ {
                    let resolved = self.subs.apply(&hir_value.type_);
                    match self.canonicalize_type(ann, *span) {
                        Ok(canonical) => {
                            if let Err(e) = self.subs.unify(
                                &self.source,
                                &canonical,
                                &resolved,
                                *span,
                            ) {
                                self.errors.push(*e);
                            }
                        }
                        Err(error) => self.errors.push(*error),
                    }
                }

                let value_type = self.subs.apply(&hir_value.type_);
                let scheme = TypeScheme {
                    type_: value_type.clone(),
                    mutable: *mutable,
                };
                self.scope.bind(name, scheme);

                Ok(HirStatement {
                    kind: HirStatementKind::Let {
                        span: *span,
                        name: name.clone(),
                        mutable: *mutable,
                        type_: value_type,
                        value: hir_value,
                    },
                })
            }
            Statement::Expression(expr) => {
                let hir_expr = self.infer_expr(expr, signatures)?;
                Ok(HirStatement {
                    kind: HirStatementKind::Expr(hir_expr, expr.span()),
                })
            }
            Statement::Return(expr, span) => {
                let hir_expr = expr
                    .as_ref()
                    .map(|e| self.infer_expr(e, signatures))
                    .transpose()?;

                if let Some(return_type) = self.current_return_type.clone() {
                    match &hir_expr {
                        Some(val) => {
                            if let Err(e) =
                                self.subs
                                    .unify(&self.source, &val.type_, &return_type, *span)
                            {
                                self.errors.push(*e);
                            }
                        }
                        None => {
                            if let Err(e) = self.subs.unify(
                                &self.source,
                                &Type::Primitive(Primitive::Unit),
                                &return_type,
                                *span,
                            ) {
                                self.errors.push(*e);
                            }
                        }
                    }
                }

                Ok(HirStatement {
                    kind: HirStatementKind::Return(hir_expr, *span),
                })
            }
            Statement::Value(expr, span) => {
                let hir_expr = self.infer_expr(expr, signatures)?;
                Ok(HirStatement {
                    kind: HirStatementKind::Value(hir_expr, *span),
                })
            }
            Statement::If { .. } => {
                panic!("Stmt::If should not appear after lowering; use Expr::If");
            }
            Statement::While { .. } => {
                panic!("Stmt::While should not appear after lowering; lowered to Stmt::Loop");
            }
            Statement::Loop { span, body } => {
                self.loop_depth += 1;
                self.scope.push_scope();
                let hir_body = self.infer_block(body, signatures)?;
                self.scope.pop_scope();
                self.loop_depth -= 1;
                Ok(HirStatement {
                    kind: HirStatementKind::Loop {
                        span: *span,
                        body: hir_body,
                    },
                })
            }
            Statement::Break(span) => {
                if self.loop_depth == 0 {
                    return Err(Box::new(
                        self.source
                            .error(*span, TypeDiagnosticKind::BreakOutsideLoop),
                    ));
                }
                Ok(HirStatement {
                    kind: HirStatementKind::Break(*span),
                })
            }
            Statement::Continue(span) => {
                if self.loop_depth == 0 {
                    return Err(Box::new(
                        self.source
                            .error(*span, TypeDiagnosticKind::ContinueOutsideLoop),
                    ));
                }
                Ok(HirStatement {
                    kind: HirStatementKind::Continue(*span),
                })
            }
            Statement::Assign {
                span,
                target,
                op,
                value,
            } => {
                let hir_value = self.infer_expr(value, signatures)?;
                let target_type = self.infer_assign_target(
                    target,
                    op,
                    &hir_value.type_,
                    *span,
                    signatures,
                    value,
                )?;
                Ok(HirStatement {
                    kind: HirStatementKind::Assign {
                        span: *span,
                        target: target_type,
                        op: crate::hir::AssignOp::from_parser(op),
                        value: hir_value,
                    },
                })
            }
        }
    }

    pub(super) fn infer_assign_target(
        &mut self,
        target: &AssignTarget,
        ast_op: &AssignOp,
        value_type: &Type,
        span: SourceSpan,
        signatures: &HashMap<&str, &FunctionDef>,
        value_expr: &Expression,
    ) -> InferResult<HirAssignTarget> {
        match target {
            AssignTarget::Ident(name, name_span) => {
                let scheme = self.scope.lookup(name).cloned().ok_or_else(|| {
                    self.source.error(
                        *name_span,
                        TypeDiagnosticKind::UndefinedVariable { name: name.clone() },
                    )
                })?;
                let resolved_type = self.subs.apply(&scheme.type_);

                self.scope
                    .check_assign_mutability(&self.source, name, *name_span)?;

                if let Expression::Ref { operand, .. } = value_expr
                    && let Expression::Ident(ref_name, ref_span) = operand.as_ref()
                    && let (Some(target_depth), Some(ref_depth)) = (
                        self.scope.lookup_scope_index(name),
                        self.scope.lookup_scope_index(ref_name),
                    )
                    && ref_depth > target_depth
                {
                    return Err(Box::new(self.source.error(
                        *ref_span,
                        TypeDiagnosticKind::InnerScopeRef {
                            name: ref_name.clone(),
                        },
                    )));
                }

                if let Type::Ref(inner) = &resolved_type {
                    if *ast_op == AssignOp::Eq && matches!(value_expr, Expression::Ref { .. }) {
                        if let Some(e) =
                            self.check_assign_type_change(inner, value_type, *name_span)
                        {
                            self.errors.push(*e);
                            return Ok(HirAssignTarget::Ident(name.clone(), *name_span));
                        }
                        if let Err(e) =
                            self.subs
                                .unify(&self.source, value_type, &resolved_type, span)
                        {
                            self.errors.push(*e);
                        }
                        return Ok(HirAssignTarget::Ident(name.clone(), *name_span));
                    }
                    if let Some(e) = self.check_assign_type_change(inner, value_type, *name_span) {
                        self.errors.push(*e);
                        return Ok(HirAssignTarget::Deref(
                            Box::new(HirExpression {
                                kind: HirExpressionKind::Ident(name.clone(), *name_span),
                                type_: scheme.type_,
                            }),
                            *name_span,
                        ));
                    }
                    if let Err(e) = self.subs.unify(&self.source, value_type, inner, span) {
                        self.errors.push(*e);
                    }
                    return Ok(HirAssignTarget::Deref(
                        Box::new(HirExpression {
                            kind: HirExpressionKind::Ident(name.clone(), *name_span),
                            type_: scheme.type_,
                        }),
                        *name_span,
                    ));
                }

                if let Some(e) =
                    self.check_assign_type_change(&resolved_type, value_type, *name_span)
                {
                    self.errors.push(*e);
                    return Ok(HirAssignTarget::Ident(name.clone(), *name_span));
                }
                if let Err(e) = self
                    .subs
                    .unify(&self.source, value_type, &resolved_type, span)
                {
                    self.errors.push(*e);
                }
                Ok(HirAssignTarget::Ident(name.clone(), *name_span))
            }
            AssignTarget::Index {
                span: index_span,
                array,
                index,
            } => {
                let hir_array = self.infer_expr(array, signatures)?;
                let hir_index = self.infer_expr(index, signatures)?;
                Ok(HirAssignTarget::Index {
                    span: *index_span,
                    array: Box::new(hir_array),
                    index: Box::new(hir_index),
                })
            }
            AssignTarget::Field {
                span: field_span,
                object,
                name,
            } => {
                let hir_object = self.infer_expr(object, signatures)?;
                Ok(HirAssignTarget::Field {
                    span: *field_span,
                    object: Box::new(hir_object),
                    name: name.clone(),
                })
            }
        }
    }

    fn check_assign_type_change(
        &self,
        target_type: &Type,
        value_type: &Type,
        span: SourceSpan,
    ) -> Option<Box<TypeDiagnostic>> {
        let resolved = self.subs.resolve(target_type);
        if let Type::Var(id) = &resolved
            && !self.subs.subs.contains_key(id)
        {
            let resolved_value = self.subs.resolve(value_type);
            let inner_value = match &resolved_value {
                Type::Ref(inner) => self.subs.resolve(inner),
                other => other.clone(),
            };
            if !matches!(&inner_value, Type::Var(_)) {
                let is_float = self.subs.float_vars.contains(id);
                let compatible = if is_float {
                    inner_value.is_float()
                } else {
                    inner_value.is_numeric()
                };
                if !compatible {
                    let default_type = if is_float { "float64" } else { "int64" };
                    return Some(Box::new(self.source.error(
                        span,
                        TypeDiagnosticKind::AssignTypeMismatch {
                            expected: default_type.to_string(),
                            found: value_type.clone(),
                        },
                    )));
                }
            }
        }
        None
    }
}
