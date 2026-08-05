use vinyl_parser::ast::types::Primitive;

use crate::hir::{
    HirAssignTarget, HirExpression, HirExpressionKind, HirMatchArm, HirPattern, HirPatternKind,
    HirStatement, HirStatementKind, Type,
};
use crate::infer::InferState;

impl InferState {
    pub(super) fn resolve_hir_type(&self, t: &Type) -> Type {
        match t {
            Type::Var(id) => {
                if let Some(resolved) = self.subs.subs.get(id) {
                    self.resolve_hir_type(resolved)
                } else if self.subs.float_vars.contains(id) {
                    Type::Primitive(Primitive::Float64)
                } else {
                    Type::Primitive(Primitive::Int64)
                }
            }
            Type::Ref(inner) => Type::Ref(Box::new(self.resolve_hir_type(inner))),
            Type::Array { element, size } => Type::Array {
                element: Box::new(self.resolve_hir_type(element)),
                size: *size,
            },
            Type::Generic { name, args } => Type::Generic {
                name: name.clone(),
                args: args.iter().map(|a| self.resolve_hir_type(a)).collect(),
            },
            Type::Tuple(elements) => {
                Type::Tuple(elements.iter().map(|e| self.resolve_hir_type(e)).collect())
            }
            other => other.clone(),
        }
    }

    pub(super) fn resolve_hir_expr(&self, expr: &HirExpression) -> HirExpression {
        HirExpression {
            kind: match &expr.kind {
                HirExpressionKind::Binary {
                    span,
                    left,
                    op,
                    right,
                } => HirExpressionKind::Binary {
                    span: *span,
                    left: Box::new(self.resolve_hir_expr(left)),
                    op: op.clone(),
                    right: Box::new(self.resolve_hir_expr(right)),
                },
                HirExpressionKind::Unary { span, op, operand } => HirExpressionKind::Unary {
                    span: *span,
                    op: op.clone(),
                    operand: Box::new(self.resolve_hir_expr(operand)),
                },
                HirExpressionKind::Call {
                    span,
                    function,
                    args,
                } => HirExpressionKind::Call {
                    span: *span,
                    function: Box::new(self.resolve_hir_expr(function)),
                    args: args.iter().map(|a| self.resolve_hir_expr(a)).collect(),
                },
                HirExpressionKind::Block(stmts, span) => HirExpressionKind::Block(
                    stmts.iter().map(|s| self.resolve_hir_stmt(s)).collect(),
                    *span,
                ),
                HirExpressionKind::Index { span, array, index } => HirExpressionKind::Index {
                    span: *span,
                    array: Box::new(self.resolve_hir_expr(array)),
                    index: Box::new(self.resolve_hir_expr(index)),
                },
                HirExpressionKind::Array(elements, span) => HirExpressionKind::Array(
                    elements.iter().map(|e| self.resolve_hir_expr(e)).collect(),
                    *span,
                ),
                HirExpressionKind::If {
                    span,
                    condition,
                    then_block,
                    else_if,
                    else_block,
                } => HirExpressionKind::If {
                    span: *span,
                    condition: Box::new(self.resolve_hir_expr(condition)),
                    then_block: then_block
                        .iter()
                        .map(|s| self.resolve_hir_stmt(s))
                        .collect(),
                    else_if: else_if
                        .iter()
                        .map(|(c, b)| {
                            (
                                self.resolve_hir_expr(c),
                                b.iter().map(|s| self.resolve_hir_stmt(s)).collect(),
                            )
                        })
                        .collect(),
                    else_block: else_block
                        .as_ref()
                        .map(|b| b.iter().map(|s| self.resolve_hir_stmt(s)).collect()),
                },
                HirExpressionKind::Unit(span) => HirExpressionKind::Unit(*span),
                HirExpressionKind::Ref(expr, span) => {
                    HirExpressionKind::Ref(Box::new(self.resolve_hir_expr(expr)), *span)
                }
                HirExpressionKind::Tuple(elements, span) => HirExpressionKind::Tuple(
                    elements.iter().map(|e| self.resolve_hir_expr(e)).collect(),
                    *span,
                ),
                HirExpressionKind::FieldAccess { span, object, name } => {
                    HirExpressionKind::FieldAccess {
                        span: *span,
                        object: Box::new(self.resolve_hir_expr(object)),
                        name: name.clone(),
                    }
                }
                HirExpressionKind::EnumVariant {
                    span,
                    type_name,
                    variant_index,
                    payload,
                } => HirExpressionKind::EnumVariant {
                    span: *span,
                    type_name: type_name.clone(),
                    variant_index: *variant_index,
                    payload: payload.iter().map(|e| self.resolve_hir_expr(e)).collect(),
                },
                HirExpressionKind::Struct {
                    span,
                    type_name,
                    fields,
                } => HirExpressionKind::Struct {
                    span: *span,
                    type_name: type_name.clone(),
                    fields: fields
                        .iter()
                        .map(|(n, e)| (n.clone(), self.resolve_hir_expr(e)))
                        .collect(),
                },
                HirExpressionKind::Match { span, value, arms } => HirExpressionKind::Match {
                    span: *span,
                    value: Box::new(self.resolve_hir_expr(value)),
                    arms: arms
                        .iter()
                        .map(|arm| HirMatchArm {
                            span: arm.span,
                            pattern: self.resolve_hir_pattern(&arm.pattern),
                            guard: arm
                                .guard
                                .as_ref()
                                .map(|g| Box::new(self.resolve_hir_expr(g))),
                            body: arm
                                .body
                                .iter()
                                .map(|s| self.resolve_hir_stmt(s))
                                .collect(),
                        })
                        .collect(),
                },
                other => other.clone(),
            },
            type_: self.resolve_hir_type(&expr.type_),
        }
    }

    pub(super) fn resolve_hir_pattern(&self, pattern: &HirPattern) -> HirPattern {
        HirPattern {
            kind: match &pattern.kind {
                HirPatternKind::Wildcard(span) => HirPatternKind::Wildcard(*span),
                HirPatternKind::Ident { span, name } => HirPatternKind::Ident {
                    span: *span,
                    name: name.clone(),
                },
                HirPatternKind::Literal { span, value } => HirPatternKind::Literal {
                    span: *span,
                    value: value.clone(),
                },
                HirPatternKind::Struct {
                    span,
                    type_name,
                    fields,
                } => HirPatternKind::Struct {
                    span: *span,
                    type_name: type_name.clone(),
                    fields: fields
                        .iter()
                        .map(|(n, p)| (n.clone(), self.resolve_hir_pattern(p)))
                        .collect(),
                },
                HirPatternKind::Tuple { span, elements } => HirPatternKind::Tuple {
                    span: *span,
                    elements: elements
                        .iter()
                        .map(|p| self.resolve_hir_pattern(p))
                        .collect(),
                },
                HirPatternKind::EnumVariant {
                    span,
                    type_name,
                    variant_index,
                    patterns,
                } => HirPatternKind::EnumVariant {
                    span: *span,
                    type_name: type_name.clone(),
                    variant_index: *variant_index,
                    patterns: patterns
                        .iter()
                        .map(|p| self.resolve_hir_pattern(p))
                        .collect(),
                },
            },
            type_: self.resolve_hir_type(&pattern.type_),
        }
    }

    pub(super) fn resolve_hir_stmt(&self, stmt: &HirStatement) -> HirStatement {
        HirStatement {
            kind: match &stmt.kind {
                HirStatementKind::Let {
                    span,
                    name,
                    mutable,
                    type_,
                    value,
                } => HirStatementKind::Let {
                    span: *span,
                    name: name.clone(),
                    mutable: *mutable,
                    type_: self.resolve_hir_type(type_),
                    value: self.resolve_hir_expr(value),
                },
                HirStatementKind::Expr(expr, span) => {
                    HirStatementKind::Expr(self.resolve_hir_expr(expr), *span)
                }
                HirStatementKind::Return(expr, span) => {
                    HirStatementKind::Return(expr.as_ref().map(|e| self.resolve_hir_expr(e)), *span)
                }
                HirStatementKind::Value(expr, span) => {
                    HirStatementKind::Value(self.resolve_hir_expr(expr), *span)
                }
                HirStatementKind::Loop { span, body } => HirStatementKind::Loop {
                    span: *span,
                    body: body.iter().map(|s| self.resolve_hir_stmt(s)).collect(),
                },
                HirStatementKind::Break(span) => HirStatementKind::Break(*span),
                HirStatementKind::Continue(span) => HirStatementKind::Continue(*span),
                HirStatementKind::Assign {
                    span,
                    target,
                    op,
                    value,
                } => HirStatementKind::Assign {
                    span: *span,
                    target: self.resolve_hir_assign_target(target),
                    op: op.clone(),
                    value: self.resolve_hir_expr(value),
                },
            },
        }
    }

    pub(super) fn resolve_hir_assign_target(&self, target: &HirAssignTarget) -> HirAssignTarget {
        match target {
            HirAssignTarget::Ident(name, span) => HirAssignTarget::Ident(name.clone(), *span),
            HirAssignTarget::Index { span, array, index } => HirAssignTarget::Index {
                span: *span,
                array: Box::new(self.resolve_hir_expr(array)),
                index: Box::new(self.resolve_hir_expr(index)),
            },
            HirAssignTarget::Field { span, object, name } => HirAssignTarget::Field {
                span: *span,
                object: Box::new(self.resolve_hir_expr(object)),
                name: name.clone(),
            },
            HirAssignTarget::Deref(expr, span) => {
                HirAssignTarget::Deref(Box::new(self.resolve_hir_expr(expr)), *span)
            }
        }
    }

    pub(super) fn resolve_hir_stmts(&self, stmts: Vec<HirStatement>) -> Vec<HirStatement> {
        stmts.iter().map(|s| self.resolve_hir_stmt(s)).collect()
    }
}
