use vinyl_parser::ast::operator::BinaryOp;
use vinyl_parser::ast::types::Primitive;

use crate::error::{TypeDiagnostic, TypeDiagnosticKind};
use crate::hir::{
    AssignOp, HirExpression, HirExpressionKind, HirStatement, HirStatementKind, Type,
};
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
            HirStatementKind::Assign {
                span, op, value, ..
            } => {
                if *op == AssignOp::PowEq {
                    self.check_pow_negative_exponent(&value.type_, value, *span, errors);
                }
                self.validate_literal_types_expr(value, errors);
            }
        }
    }

    fn check_pow_negative_exponent(
        &self,
        base_type: &Type,
        exponent: &HirExpression,
        span: miette::SourceSpan,
        errors: &mut Vec<TypeDiagnostic>,
    ) {
        if !base_type.is_float()
            && let HirExpressionKind::Int(value, _) = &exponent.kind
            && *value < 0
        {
            errors.push(self.source.error(
                span,
                TypeDiagnosticKind::PowNegativeExponent { value: *value },
            ));
        }
    }

    fn validate_literal_types_expr(&self, expr: &HirExpression, errors: &mut Vec<TypeDiagnostic>) {
        match &expr.kind {
            HirExpressionKind::Int(value, span) => {
                if !expr.type_.is_numeric() {
                    errors.push(self.source.error(
                        *span,
                        TypeDiagnosticKind::IntLiteralMismatch {
                            found: expr.type_.clone(),
                        },
                    ));
                } else if let Some((min, max)) = int_range(&expr.type_)
                    && (*value < min || *value > max)
                {
                    errors.push(self.source.error(
                        *span,
                        TypeDiagnosticKind::IntLiteralOutOfRange {
                            value: *value,
                            found: expr.type_.clone(),
                        },
                    ));
                }
            }
            HirExpressionKind::UInt(value, span) => {
                if !expr.type_.is_uint() {
                    errors.push(self.source.error(
                        *span,
                        TypeDiagnosticKind::UIntLiteralMismatch {
                            found: expr.type_.clone(),
                        },
                    ));
                } else if let Some(max) = uint_range(&expr.type_)
                    && *value > max
                {
                    errors.push(self.source.error(
                        *span,
                        TypeDiagnosticKind::UIntLiteralOutOfRange {
                            value: *value,
                            found: expr.type_.clone(),
                        },
                    ));
                }
            }
            HirExpressionKind::Float(value, span) => {
                if !expr.type_.is_float() {
                    errors.push(self.source.error(
                        *span,
                        TypeDiagnosticKind::FloatLiteralMismatch {
                            found: expr.type_.clone(),
                        },
                    ));
                } else if !float_in_range(&expr.type_, *value) {
                    errors.push(self.source.error(
                        *span,
                        TypeDiagnosticKind::FloatLiteralOutOfRange {
                            value: *value,
                            found: expr.type_.clone(),
                        },
                    ));
                }
            }
            HirExpressionKind::Binary {
                span,
                op,
                left,
                right,
                ..
            } => {
                if *op == BinaryOp::Pow {
                    self.check_pow_negative_exponent(&left.type_, right, *span, errors);
                }
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
            HirExpressionKind::Match { value, arms, .. } => {
                self.validate_literal_types_expr(value, errors);
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        self.validate_literal_types_expr(guard, errors);
                    }
                    self.validate_literal_types(&arm.body, errors);
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

fn int_range(t: &Type) -> Option<(i128, i128)> {
    let p = t.as_primitive()?;
    Some(match p {
        Primitive::Int8 => (i8::MIN as i128, i8::MAX as i128),
        Primitive::Int16 => (i16::MIN as i128, i16::MAX as i128),
        Primitive::Int32 => (i32::MIN as i128, i32::MAX as i128),
        Primitive::Int64 => (i64::MIN as i128, i64::MAX as i128),
        Primitive::Int128 => (i128::MIN, i128::MAX),
        Primitive::ISize => (isize::MIN as i128, isize::MAX as i128),
        Primitive::UInt8 => (0, u8::MAX as i128),
        Primitive::UInt16 => (0, u16::MAX as i128),
        Primitive::UInt32 => (0, u32::MAX as i128),
        Primitive::UInt64 => (0, u64::MAX as i128),
        Primitive::UInt128 => (0, i128::MAX),
        Primitive::USize => (0, usize::MAX as i128),
        _ => return None,
    })
}

fn uint_range(t: &Type) -> Option<u128> {
    let p = t.as_primitive()?;
    Some(match p {
        Primitive::UInt8 => u8::MAX as u128,
        Primitive::UInt16 => u16::MAX as u128,
        Primitive::UInt32 => u32::MAX as u128,
        Primitive::UInt64 => u64::MAX as u128,
        Primitive::UInt128 => u128::MAX,
        Primitive::USize => usize::MAX as u128,
        _ => return None,
    })
}

fn float_in_range(t: &Type, value: f64) -> bool {
    if !value.is_finite() {
        return false;
    }
    match t.as_primitive() {
        Some(Primitive::Float32) => value.abs() <= f32::MAX as f64,
        _ => true,
    }
}
