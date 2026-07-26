use crate::error::TypeError;
use crate::hir::{HirExpression, HirExpressionKind, HirStatement, HirStatementKind};
use crate::infer::InferState;

impl InferState {
    pub(super) fn validate_literal_types_stmt(
        &self,
        stmt: &HirStatement,
        errors: &mut Vec<TypeError>,
    ) {
        match &stmt.kind {
            HirStatementKind::Let { value, .. } => self.validate_literal_types_expr(value, errors),
            HirStatementKind::Expr(expr, _) => self.validate_literal_types_expr(expr, errors),
            HirStatementKind::Return(expr, _) => {
                if let Some(e) = expr {
                    self.validate_literal_types_expr(e, errors);
                }
            }
            HirStatementKind::Value(expr, _) => self.validate_literal_types_expr(expr, errors),
            HirStatementKind::Loop { body, .. } => {
                self.validate_literal_types(body, errors);
            }
            HirStatementKind::Break(_) | HirStatementKind::Continue(_) => {}
            HirStatementKind::Assign { value, .. } => {
                self.validate_literal_types_expr(value, errors);
            }
        }
    }

    fn validate_literal_types_expr(&self, expr: &HirExpression, errors: &mut Vec<TypeError>) {
        match &expr.kind {
            HirExpressionKind::Int(_, span) if !expr.type_.is_numeric() => {
                errors.push(self.source.error(
                    *span,
                    format!(
                        "integer literal must be a numeric type, found `{}`",
                        expr.type_
                    ),
                ));
            }
            HirExpressionKind::Float(_, span) if !expr.type_.is_float() => {
                errors.push(self.source.error(
                    *span,
                    format!("float literal must be a float type, found `{}`", expr.type_),
                ));
            }
            HirExpressionKind::Binary { left, right, .. } => {
                self.validate_literal_types_expr(left, errors);
                self.validate_literal_types_expr(right, errors);
            }
            HirExpressionKind::Unary { operand, .. } => {
                self.validate_literal_types_expr(operand, errors);
            }
            HirExpressionKind::Call { function, args, .. } => {
                self.validate_literal_types_expr(function, errors);
                for arg in args {
                    self.validate_literal_types_expr(arg, errors);
                }
            }
            HirExpressionKind::Block(stmts, _) => self.validate_literal_types(stmts, errors),
            HirExpressionKind::Index { span, array, index } => {
                self.validate_literal_types_expr(array, errors);
                self.validate_literal_types_expr(index, errors);
                if !index.type_.is_int() && !index.type_.is_uint() {
                    errors.push(self.source.error(
                        *span,
                        format!("index type must be an integer, found `{}`", index.type_),
                    ));
                }
            }
            HirExpressionKind::Array(elements, _) => {
                for e in elements {
                    self.validate_literal_types_expr(e, errors);
                }
            }
            HirExpressionKind::If {
                condition,
                then_block,
                else_if,
                else_block,
                ..
            } => {
                self.validate_literal_types_expr(condition, errors);
                self.validate_literal_types(then_block, errors);
                for (c, b) in else_if {
                    self.validate_literal_types_expr(c, errors);
                    self.validate_literal_types(b, errors);
                }
                if let Some(b) = else_block {
                    self.validate_literal_types(b, errors);
                }
            }
            _ => {}
        }
    }

    pub(super) fn collect_literal_type_errors(&self, stmts: &[HirStatement]) -> Vec<TypeError> {
        let mut errors = Vec::new();
        self.validate_literal_types(stmts, &mut errors);
        errors
    }

    fn validate_literal_types(&self, stmts: &[HirStatement], errors: &mut Vec<TypeError>) {
        for stmt in stmts {
            self.validate_literal_types_stmt(stmt, errors);
        }
    }
}
