use std::collections::HashSet;

use vinyl_typecheck::hir::{
    HirAssignTarget, HirExpression, HirExpressionKind, HirStatement, HirStatementKind,
};

pub fn prescan_function_body(body: &[HirStatement]) -> HashSet<String> {
    let mut refed = HashSet::new();
    prescan_stmts(body, &mut refed);
    refed
}

fn prescan_stmts(stmts: &[HirStatement], refed: &mut HashSet<String>) {
    for stmt in stmts {
        match &stmt.kind {
            HirStatementKind::Let { value, .. } => prescan_expr(value, refed),
            HirStatementKind::Expr(e) | HirStatementKind::Value(e) => prescan_expr(e, refed),
            HirStatementKind::Return(Some(e)) => prescan_expr(e, refed),
            HirStatementKind::Return(None) => {}
            HirStatementKind::Loop { body } => prescan_stmts(body, refed),
            HirStatementKind::Break | HirStatementKind::Continue => {}
            HirStatementKind::Assign { target, value, .. } => {
                if let HirAssignTarget::Deref(e) = target {
                    prescan_expr(e, refed);
                }
                prescan_expr(value, refed);
            }
        }
    }
}

fn prescan_expr(expr: &HirExpression, refed: &mut HashSet<String>) {
    match &expr.kind {
        HirExpressionKind::Ident(_name) => {}
        HirExpressionKind::Ref(inner) => {
            if let HirExpression {
                kind: HirExpressionKind::Ident(name),
                ..
            } = inner.as_ref()
            {
                refed.insert(name.clone());
            }
            prescan_expr(inner, refed);
        }
        HirExpressionKind::Unary { operand, .. } => prescan_expr(operand, refed),
        HirExpressionKind::Binary { left, right, .. } => {
            prescan_expr(left, refed);
            prescan_expr(right, refed);
        }
        HirExpressionKind::Call { function, args } => {
            prescan_expr(function, refed);
            for arg in args {
                prescan_expr(arg, refed);
            }
        }
        HirExpressionKind::Block(stmts) => prescan_stmts(stmts, refed),
        HirExpressionKind::Array(elements) => {
            for elem in elements {
                prescan_expr(elem, refed);
            }
        }
        HirExpressionKind::Index { array, index, .. } => {
            prescan_expr(array, refed);
            prescan_expr(index, refed);
        }
        HirExpressionKind::If {
            condition,
            then_block,
            else_if,
            else_block,
        } => {
            prescan_expr(condition, refed);
            prescan_stmts(then_block, refed);
            for (cond, block) in else_if {
                prescan_expr(cond, refed);
                prescan_stmts(block, refed);
            }
            if let Some(block) = else_block {
                prescan_stmts(block, refed);
            }
        }
        HirExpressionKind::Tuple(elements, _) => {
            for elem in elements {
                prescan_expr(elem, refed);
            }
        }
        HirExpressionKind::EnumVariant { payload, .. } => {
            for elem in payload {
                prescan_expr(elem, refed);
            }
        }
        HirExpressionKind::Struct { fields, .. } => {
            for (_, expr) in fields {
                prescan_expr(expr, refed);
            }
        }
        HirExpressionKind::FieldAccess { object, .. } => {
            prescan_expr(object, refed);
        }
        HirExpressionKind::Int(..)
        | HirExpressionKind::Float(..)
        | HirExpressionKind::String(..)
        | HirExpressionKind::Bool(..)
        | HirExpressionKind::Unit
        | HirExpressionKind::Char(..) => {}
    }
}
