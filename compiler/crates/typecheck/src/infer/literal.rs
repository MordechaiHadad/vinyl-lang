use crate::error::{TypeDiagnostic, TypeDiagnosticKind};
use crate::hir::{HirExpression, HirExpressionKind, HirStatement, HirStatementKind};
use crate::infer::InferState;

impl InferState {
    pub(super) fn validate_literal_types_stmt(
        &self,
        stmt: &HirStatement,
        errors: &mut Vec<TypeDiagnostic>,
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

    fn validate_literal_types_expr(&self, expr: &HirExpression, errors: &mut Vec<TypeDiagnostic>) {
        match &expr.kind {
            HirExpressionKind::Int(_, span) if !expr.type_.is_numeric() => {
                errors.push(self.source.error(
                    *span,
                    TypeDiagnosticKind::IntLiteralMismatch {
                        found: expr.type_.clone(),
                    },
                ));
            }
            HirExpressionKind::Float(_, span) if !expr.type_.is_float() => {
                errors.push(self.source.error(
                    *span,
                    TypeDiagnosticKind::FloatLiteralMismatch {
                        found: expr.type_.clone(),
                    },
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
                        TypeDiagnosticKind::IndexMustBeInteger {
                            found: index.type_.clone(),
                        },
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

    pub(super) fn collect_literal_type_errors(
        &self,
        stmts: &[HirStatement],
    ) -> Vec<TypeDiagnostic> {
        let mut errors = Vec::new();
        self.validate_literal_types(stmts, &mut errors);
        errors
    }

    fn validate_literal_types(&self, stmts: &[HirStatement], errors: &mut Vec<TypeDiagnostic>) {
        for stmt in stmts {
            self.validate_literal_types_stmt(stmt, errors);
        }
    }
}
