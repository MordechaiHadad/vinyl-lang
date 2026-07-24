use vinyl_parser::ast::types::Primitive;

use crate::hir::{
    HirAssignTarget, HirExpression, HirExpressionKind, HirStatement, HirStatementKind, Type,
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
                HirExpressionKind::Binary { left, op, right } => HirExpressionKind::Binary {
                    left: Box::new(self.resolve_hir_expr(left)),
                    op: op.clone(),
                    right: Box::new(self.resolve_hir_expr(right)),
                },
                HirExpressionKind::Unary { op, operand } => HirExpressionKind::Unary {
                    op: op.clone(),
                    operand: Box::new(self.resolve_hir_expr(operand)),
                },
                HirExpressionKind::Call { function, args } => HirExpressionKind::Call {
                    function: Box::new(self.resolve_hir_expr(function)),
                    args: args.iter().map(|a| self.resolve_hir_expr(a)).collect(),
                },
                HirExpressionKind::Block(stmts) => {
                    HirExpressionKind::Block(stmts.iter().map(|s| self.resolve_hir_stmt(s)).collect())
                }
                HirExpressionKind::Index { span, array, index } => HirExpressionKind::Index {
                    span: *span,
                    array: Box::new(self.resolve_hir_expr(array)),
                    index: Box::new(self.resolve_hir_expr(index)),
                },
                HirExpressionKind::Array(elements) => {
                    HirExpressionKind::Array(elements.iter().map(|e| self.resolve_hir_expr(e)).collect())
                }
                HirExpressionKind::If {
                    condition,
                    then_block,
                    else_if,
                    else_block,
                } => HirExpressionKind::If {
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
                HirExpressionKind::Unit => HirExpressionKind::Unit,
                HirExpressionKind::Ref(expr) => HirExpressionKind::Ref(Box::new(self.resolve_hir_expr(expr))),
                HirExpressionKind::Tuple(elements, span) => HirExpressionKind::Tuple(
                    elements.iter().map(|e| self.resolve_hir_expr(e)).collect(),
                    *span,
                ),
                HirExpressionKind::FieldAccess { span, object, name } => HirExpressionKind::FieldAccess {
                    span: *span,
                    object: Box::new(self.resolve_hir_expr(object)),
                    name: name.clone(),
                },
                HirExpressionKind::EnumVariant {
                    type_name,
                    variant_index,
                    payload,
                } => HirExpressionKind::EnumVariant {
                    type_name: type_name.clone(),
                    variant_index: *variant_index,
                    payload: payload.iter().map(|e| self.resolve_hir_expr(e)).collect(),
                },
                HirExpressionKind::Struct { type_name, fields } => {
                    HirExpressionKind::Struct {
                        type_name: type_name.clone(),
                        fields: fields
                            .iter()
                            .map(|(n, e)| (n.clone(), self.resolve_hir_expr(e)))
                            .collect(),
                    }
                }
                other => other.clone(),
            },
            type_: self.resolve_hir_type(&expr.type_),
        }
    }

    pub(super) fn resolve_hir_stmt(&self, stmt: &HirStatement) -> HirStatement {
        HirStatement {
            kind: match &stmt.kind {
                HirStatementKind::Let {
                    name,
                    mutable,
                    type_,
                    value,
                } => HirStatementKind::Let {
                    name: name.clone(),
                    mutable: *mutable,
                    type_: self.resolve_hir_type(type_),
                    value: self.resolve_hir_expr(value),
                },
                HirStatementKind::Expr(expr) => HirStatementKind::Expr(self.resolve_hir_expr(expr)),
                HirStatementKind::Return(expr) => {
                    HirStatementKind::Return(expr.as_ref().map(|e| self.resolve_hir_expr(e)))
                }
                HirStatementKind::Value(expr) => {
                    HirStatementKind::Value(self.resolve_hir_expr(expr))
                }
                HirStatementKind::Loop { body } => HirStatementKind::Loop {
                    body: body.iter().map(|s| self.resolve_hir_stmt(s)).collect(),
                },
                HirStatementKind::Break => HirStatementKind::Break,
                HirStatementKind::Continue => HirStatementKind::Continue,
                HirStatementKind::Assign { target, op, value } => HirStatementKind::Assign {
                    target: self.resolve_hir_assign_target(target),
                    op: op.clone(),
                    value: self.resolve_hir_expr(value),
                },
            },
        }
    }

    pub(super) fn resolve_hir_assign_target(&self, target: &HirAssignTarget) -> HirAssignTarget {
        match target {
            HirAssignTarget::Ident(name) => HirAssignTarget::Ident(name.clone()),
            HirAssignTarget::Index { array, index } => HirAssignTarget::Index {
                array: Box::new(self.resolve_hir_expr(array)),
                index: Box::new(self.resolve_hir_expr(index)),
            },
            HirAssignTarget::Field { object, name } => HirAssignTarget::Field {
                object: Box::new(self.resolve_hir_expr(object)),
                name: name.clone(),
            },
            HirAssignTarget::Deref(expr) => {
                HirAssignTarget::Deref(Box::new(self.resolve_hir_expr(expr)))
            }
        }
    }

    pub(super) fn resolve_hir_stmts(&self, stmts: Vec<HirStatement>) -> Vec<HirStatement> {
        stmts.iter().map(|s| self.resolve_hir_stmt(s)).collect()
    }
}
