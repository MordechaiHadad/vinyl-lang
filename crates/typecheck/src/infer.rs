use miette::{Diagnostic, NamedSource, SourceSpan};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;
use vinyl_parser::ast::expression::Expression;
use vinyl_parser::ast::item::{EnumVariantData, FunctionDef, Item};
use vinyl_parser::ast::operator::{AssignOp, BinaryOp, UnaryOp};
use vinyl_parser::ast::statement::{AssignTarget, Statement};
use vinyl_parser::ast::types::Primitive;

use crate::hir::{
    AssignOp as HirAssignOp, HirAssignTarget, HirEnum, HirEnumVariant, HirEnumVariantData, HirExpr,
    HirExprKind, HirField, HirFunction, HirItem, HirItemKind, HirParam, HirStatement,
    HirStatementKind, HirStruct, HirTupleStruct, Type,
};

#[derive(Debug, Diagnostic)]
#[diagnostic()]
pub struct TypeError {
    pub message: String,
    #[source_code]
    pub source: NamedSource<String>,
    #[label]
    pub span: SourceSpan,
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for TypeError {}

#[derive(Debug, Diagnostic)]
#[diagnostic(severity(Warning))]
pub struct CompileWarning {
    pub message: String,
    #[source_code]
    pub source: NamedSource<String>,
    #[label]
    pub span: SourceSpan,
}

impl fmt::Display for CompileWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for CompileWarning {}

pub fn typeck(
    items: &[Item],
    source: &str,
    source_name: &str,
    warnings: &mut Vec<CompileWarning>,
) -> Result<Vec<HirItem>, Vec<TypeError>> {
    let mut state = InferState::new(source, source_name);

    let signatures: HashMap<&str, &FunctionDef> = items
        .iter()
        .filter_map(|item| {
            if let Item::Function(f) = item {
                Some((f.name.as_str(), f))
            } else {
                None
            }
        })
        .collect();

    let mut hir_items: Vec<HirItem> = Vec::new();

    // First pass: register type definitions
    for item in items {
        let hir_item = match item {
            Item::Struct(s) => Some(HirItem {
                kind: HirItemKind::Struct(HirStruct {
                    name: s.name.clone(),
                    repr_c: s.attrs.iter().any(|a| a.name == "repr_c"),
                    fields: s
                        .fields
                        .iter()
                        .map(|f| HirField {
                            name: f.name.clone(),
                            type_: f.type_.clone(),
                        })
                        .collect(),
                }),
            }),
            Item::TupleStruct(t) => Some(HirItem {
                kind: HirItemKind::TupleStruct(HirTupleStruct {
                    name: t.name.clone(),
                    types: t.types.clone(),
                }),
            }),
            Item::Enum(e) => Some(HirItem {
                kind: HirItemKind::Enum(HirEnum {
                    name: e.name.clone(),
                    variants: e
                        .variants
                        .iter()
                        .map(|v| HirEnumVariant {
                            name: v.name.clone(),
                            data: v.data.as_ref().map(|d| match d {
                                EnumVariantData::Tuple(types) => {
                                    HirEnumVariantData::Tuple(types.clone())
                                }
                                EnumVariantData::Struct(fields) => HirEnumVariantData::Struct(
                                    fields
                                        .iter()
                                        .map(|f| HirField {
                                            name: f.name.clone(),
                                            type_: f.type_.clone(),
                                        })
                                        .collect(),
                                ),
                            }),
                        })
                        .collect(),
                }),
            }),
            Item::Function(_) => None,
        };
        if let Some(hir) = hir_item {
            let name = match &hir.kind {
                HirItemKind::Struct(s) => s.name.clone(),
                HirItemKind::TupleStruct(t) => t.name.clone(),
                HirItemKind::Enum(e) => e.name.clone(),
                _ => unreachable!(),
            };
            state.types.insert(name, hir.kind.clone());
            hir_items.push(hir);
        }
    }

    // Second pass: type-check functions with registered types available
    for item in items {
        if let Item::Function(f) = item {
            match state.infer_function(f, &signatures) {
                Ok(hir) => hir_items.push(HirItem {
                    kind: HirItemKind::Function(hir),
                }),
                Err(e) => state.errors.push(e),
            }
        }
    }

    warnings.append(&mut state.warnings);

    if state.errors.is_empty() {
        Ok(hir_items)
    } else {
        Err(state.errors)
    }
}

#[derive(Debug, Clone)]
struct TypeScheme {
    type_: Type,
    mutable: bool,
}

struct InferState {
    source: String,
    source_name: String,
    scopes: Vec<HashMap<String, TypeScheme>>,
    types: HashMap<String, HirItemKind>,
    errors: Vec<TypeError>,
    warnings: Vec<CompileWarning>,
    subs: HashMap<usize, Type>,
    current_return_type: Option<Type>,
    next_var: usize,
    loop_depth: usize,
    float_vars: HashSet<usize>,
}

impl InferState {
    fn new(source: &str, source_name: &str) -> Self {
        InferState {
            source: source.to_string(),
            source_name: source_name.to_string(),
            scopes: vec![HashMap::new()],
            types: HashMap::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
            subs: HashMap::new(),
            current_return_type: None,
            next_var: 0,
            loop_depth: 0,
            float_vars: HashSet::new(),
        }
    }

    fn resolve(&self, t: &Type) -> Type {
        match t {
            Type::Var(id) => {
                if let Some(resolved) = self.subs.get(id) {
                    self.resolve(resolved)
                } else {
                    t.clone()
                }
            }
            _ => t.clone(),
        }
    }

    fn apply(&self, t: &Type) -> Type {
        match t {
            Type::Var(id) => {
                if let Some(resolved) = self.subs.get(id) {
                    self.apply(resolved)
                } else {
                    Type::Var(*id)
                }
            }
            Type::Ref(inner) => Type::Ref(Box::new(self.apply(inner))),
            Type::Array { element, size } => Type::Array {
                element: Box::new(self.apply(element)),
                size: *size,
            },
            Type::Generic { name, args } => Type::Generic {
                name: name.clone(),
                args: args.iter().map(|a| self.apply(a)).collect(),
            },
            Type::Tuple(elements) => Type::Tuple(elements.iter().map(|e| self.apply(e)).collect()),
            other => other.clone(),
        }
    }

    fn occurs(&self, id: usize, t: &Type) -> bool {
        match t {
            Type::Var(vid) => *vid == id || self.subs.get(vid).is_some_and(|t| self.occurs(id, t)),
            Type::Ref(inner) => self.occurs(id, inner),
            Type::Array { element, .. } => self.occurs(id, element),
            Type::Generic { args, .. } => args.iter().any(|a| self.occurs(id, a)),
            Type::Tuple(elements) => elements.iter().any(|e| self.occurs(id, e)),
            _ => false,
        }
    }

    fn unify(&mut self, a: &Type, b: &Type, span: SourceSpan) -> Result<(), TypeError> {
        let a = self.resolve(a);
        let b = self.resolve(b);

        if a == b {
            return Ok(());
        }

        match (&a, &b) {
            (Type::Var(id_a), _) => {
                if self.occurs(*id_a, &b) {
                    return Err(self.error(span, format!("recursive type: `{a}` contains `{b}`")));
                }
                self.subs.insert(*id_a, b.clone());
                Ok(())
            }
            (_, Type::Var(id_b)) => {
                if self.occurs(*id_b, &a) {
                    return Err(self.error(span, format!("recursive type: `{b}` contains `{a}`")));
                }
                self.subs.insert(*id_b, a.clone());
                Ok(())
            }
            (Type::Primitive(p1), Type::Primitive(p2)) if p1 == p2 => Ok(()),
            (Type::Named(n1), Type::Named(n2)) if n1 == n2 => Ok(()),
            (Type::Generic { name: n1, args: a1 }, Type::Generic { name: n2, args: a2 })
                if n1 == n2 && a1.len() == a2.len() =>
            {
                for (ai, bi) in a1.iter().zip(a2) {
                    self.unify(ai, bi, span)?;
                }
                Ok(())
            }
            (Type::Ref(t1), Type::Ref(t2)) => self.unify(t1, t2, span),
            (
                Type::Array {
                    element: e1,
                    size: s1,
                },
                Type::Array {
                    element: e2,
                    size: s2,
                },
            ) if s1 == s2 => self.unify(e1, e2, span),
            (Type::Tuple(a), Type::Tuple(b)) if a.len() == b.len() => {
                for (ai, bi) in a.iter().zip(b) {
                    self.unify(ai, bi, span)?;
                }
                Ok(())
            }
            _ => Err(self.error(
                span,
                format!("type mismatch: expected `{}`, found `{}`", b, a),
            )),
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn bind(&mut self, name: &str, scheme: TypeScheme) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), scheme);
        }
    }

    fn lookup(&self, name: &str) -> Option<&TypeScheme> {
        for scope in self.scopes.iter().rev() {
            if let Some(scheme) = scope.get(name) {
                return Some(scheme);
            }
        }
        None
    }

    fn lookup_scope_index(&self, name: &str) -> Option<usize> {
        for (depth, scope) in self.scopes.iter().enumerate().rev() {
            if scope.contains_key(name) {
                return Some(depth);
            }
        }
        None
    }

    fn error(&self, span: SourceSpan, message: String) -> TypeError {
        TypeError {
            message,
            source: NamedSource::new(&self.source_name, self.source.to_string()),
            span,
        }
    }

    fn warn(&self, span: SourceSpan, message: String) -> CompileWarning {
        CompileWarning {
            message,
            source: NamedSource::new(&self.source_name, self.source.to_string()),
            span,
        }
    }

    fn infer_function(
        &mut self,
        func: &FunctionDef,
        signatures: &HashMap<&str, &FunctionDef>,
    ) -> Result<HirFunction, TypeError> {
        let mut params = Vec::new();
        for param in &func.params {
            let mutable = true;
            params.push(HirParam {
                name: param.name.clone(),
                mutable,
                type_: param.type_.clone(),
            });
            self.bind(
                &param.name,
                TypeScheme {
                    type_: param.type_.clone(),
                    mutable,
                },
            );
        }

        let mut return_type = match &func.return_type {
            Some(t) => t.clone(),
            None => self.fresh_var(),
        };

        // TODO: check that return type is not &T
        let resolved_ret = self.apply(&return_type);
        if let Type::Ref(_) = &resolved_ret {
            self.errors.push(self.error(
                func.span,
                "functions cannot return reference types".to_string(),
            ));
        }

        let prev_return = self.current_return_type.replace(return_type.clone());
        self.push_scope();
        let body = self.infer_block(&func.body, signatures)?;
        self.pop_scope();
        self.current_return_type = prev_return;

        if let Some(HirStatement {
            kind: HirStatementKind::Value(expr),
            ..
        }) = body.last()
        {
            let value_type = self.apply(&expr.type_);
            let ret_type = self.apply(&return_type);
            if let Err(e) = self.unify(&value_type, &ret_type, func.span) {
                self.errors.push(e);
            }
        }

        if !body
            .last()
            .is_some_and(|s| matches!(s.kind, HirStatementKind::Value(_)))
        {
            if let Type::Var(id) = &return_type {
                self.subs.remove(id);
            }
            return_type = Type::Primitive(Primitive::Unit);
        }
        let body = self.resolve_hir_stmts(body);

        self.errors.extend(self.collect_literal_type_errors(&body));

        Ok(HirFunction {
            name: func.name.clone(),
            params,
            return_type,
            body,
        })
    }

    fn infer_block(
        &mut self,
        stmts: &[Statement],
        signatures: &HashMap<&str, &FunctionDef>,
    ) -> Result<Vec<HirStatement>, TypeError> {
        let mut hir_stmts = Vec::new();
        let mut terminated = false;
        for stmt in stmts {
            if terminated {
                self.warnings
                    .push(self.warn(stmt.span(), "unreachable statement".to_string()));
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

    fn infer_stmt(
        &mut self,
        stmt: &Statement,
        signatures: &HashMap<&str, &FunctionDef>,
    ) -> Result<HirStatement, TypeError> {
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
                    let resolved = self.apply(&hir_value.type_);
                    if let Err(e) = self.unify(ann, &resolved, *span) {
                        self.errors.push(e);
                    }
                }

                let value_type = self.apply(&hir_value.type_);
                let scheme = TypeScheme {
                    type_: value_type.clone(),
                    mutable: *mutable,
                };
                self.bind(name, scheme);

                Ok(HirStatement {
                    kind: HirStatementKind::Let {
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
                    kind: HirStatementKind::Expr(hir_expr),
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
                            if let Err(e) = self.unify(&val.type_, &return_type, *span) {
                                self.errors.push(e);
                            }
                        }
                        None => {
                            if let Err(e) =
                                self.unify(&Type::Primitive(Primitive::Unit), &return_type, *span)
                            {
                                self.errors.push(e);
                            }
                        }
                    }
                }

                Ok(HirStatement {
                    kind: HirStatementKind::Return(hir_expr),
                })
            }
            Statement::Value(expr, _span) => {
                let hir_expr = self.infer_expr(expr, signatures)?;
                Ok(HirStatement {
                    kind: HirStatementKind::Value(hir_expr),
                })
            }
            Statement::If { .. } => {
                panic!("Stmt::If should not appear after lowering; use Expr::If");
            }
            Statement::While { .. } => {
                panic!("Stmt::While should not appear after lowering; lowered to Stmt::Loop");
            }
            Statement::Loop { span: _, body } => {
                self.loop_depth += 1;
                self.push_scope();
                let hir_body = self.infer_block(body, signatures)?;
                self.pop_scope();
                self.loop_depth -= 1;
                Ok(HirStatement {
                    kind: HirStatementKind::Loop { body: hir_body },
                })
            }
            Statement::Break(span) => {
                if self.loop_depth == 0 {
                    return Err(self.error(*span, "break outside of loop".to_string()));
                }
                Ok(HirStatement {
                    kind: HirStatementKind::Break,
                })
            }
            Statement::Continue(span) => {
                if self.loop_depth == 0 {
                    return Err(self.error(*span, "continue outside of loop".to_string()));
                }
                Ok(HirStatement {
                    kind: HirStatementKind::Continue,
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
                        target: target_type,
                        op: Self::hir_assign_op(op),
                        value: hir_value,
                    },
                })
            }
        }
    }

    fn infer_assign_target(
        &mut self,
        target: &AssignTarget,
        ast_op: &AssignOp,
        value_type: &Type,
        span: SourceSpan,
        signatures: &HashMap<&str, &FunctionDef>,
        value_expr: &Expression,
    ) -> Result<HirAssignTarget, TypeError> {
        match target {
            AssignTarget::Ident(name, name_span) => {
                let scheme = self.lookup(name).cloned().ok_or_else(|| {
                    self.error(*name_span, format!("undefined variable `{name}`"))
                })?;
                let resolved_type = self.apply(&scheme.type_);

                self.check_assign_mutability(name, *name_span)?;

                if let Expression::Ref { operand, .. } = value_expr
                    && let Expression::Ident(ref_name, ref_span) = operand.as_ref()
                    && let (Some(target_depth), Some(ref_depth)) = (
                        self.lookup_scope_index(name),
                        self.lookup_scope_index(ref_name),
                    )
                    && ref_depth > target_depth
                {
                    return Err(self.error(
                        *ref_span,
                        format!("cannot reference inner scope variable `{ref_name}`"),
                    ));
                }

                if let Type::Ref(inner) = &resolved_type {
                    if *ast_op == AssignOp::Eq && matches!(value_expr, Expression::Ref { .. }) {
                        self.unify(value_type, &resolved_type, span)?;
                        return Ok(HirAssignTarget::Ident(name.clone()));
                    }
                    self.unify(value_type, inner, span)?;
                    return Ok(HirAssignTarget::Deref(Box::new(HirExpr {
                        kind: HirExprKind::Ident(name.clone()),
                        type_: scheme.type_,
                    })));
                }

                // Direct assignment — unify value with target type
                self.unify(value_type, &resolved_type, span)?;
                Ok(HirAssignTarget::Ident(name.clone()))
            }
            AssignTarget::Index {
                span: _index_span,
                array,
                index,
            } => {
                let hir_array = self.infer_expr(array, signatures)?;
                let hir_index = self.infer_expr(index, signatures)?;
                Ok(HirAssignTarget::Index {
                    array: Box::new(hir_array),
                    index: Box::new(hir_index),
                })
            }
            AssignTarget::Field {
                span: _field_span,
                object,
                name,
            } => {
                let hir_object = self.infer_expr(object, signatures)?;
                Ok(HirAssignTarget::Field {
                    object: Box::new(hir_object),
                    name: name.clone(),
                })
            }
        }
    }

    fn check_assign_mutability(&self, name: &str, span: SourceSpan) -> Result<(), TypeError> {
        for scope in self.scopes.iter().rev() {
            if let Some(scheme) = scope.get(name) {
                if !scheme.mutable {
                    return Err(self.error(
                        span,
                        format!("cannot assign to immutable variable `{name}`"),
                    ));
                }
                return Ok(());
            }
        }
        Err(self.error(span, format!("variable `{name}` not found")))
    }

    fn infer_expr(
        &mut self,
        expr: &Expression,
        signatures: &HashMap<&str, &FunctionDef>,
    ) -> Result<HirExpr, TypeError> {
        match expr {
            Expression::Int(v, span) => Ok(HirExpr {
                kind: HirExprKind::Int(*v, *span),
                type_: self.fresh_var(),
            }),
            Expression::Float(v, span) => {
                let var = self.fresh_var();
                if let Type::Var(id) = &var {
                    self.float_vars.insert(*id);
                }
                Ok(HirExpr {
                    kind: HirExprKind::Float(*v, *span),
                    type_: var,
                })
            }
            Expression::String(s, _) => Ok(HirExpr {
                kind: HirExprKind::String(s.clone()),
                type_: Type::Primitive(Primitive::String),
            }),
            Expression::Unit(_) => Ok(HirExpr {
                kind: HirExprKind::Unit,
                type_: Type::Primitive(Primitive::Unit),
            }),
            Expression::Bool(b, _) => Ok(HirExpr {
                kind: HirExprKind::Bool(*b),
                type_: Type::Primitive(Primitive::Bool),
            }),
            Expression::Char(c, _) => Ok(HirExpr {
                kind: HirExprKind::Char(*c),
                type_: Type::Primitive(Primitive::Char),
            }),
            Expression::Ident(name, span) => {
                let scheme = self.lookup(name).cloned();
                match scheme {
                    Some(scheme) => {
                        let resolved = self.apply(&scheme.type_);
                        if let Type::Ref(inner) = &resolved {
                            Ok(HirExpr {
                                kind: HirExprKind::Ident(name.clone()),
                                type_: *inner.clone(),
                            })
                        } else {
                            Ok(HirExpr {
                                kind: HirExprKind::Ident(name.clone()),
                                type_: scheme.type_,
                            })
                        }
                    }
                    None if signatures.contains_key(name.as_str()) => Ok(HirExpr {
                        kind: HirExprKind::Ident(name.clone()),
                        type_: Type::Primitive(Primitive::Unit),
                    }),
                    None => Err(self.error(*span, format!("undefined variable `{name}`"))),
                }
            }
            Expression::Binary {
                span,
                left,
                op,
                right,
            } => {
                let left_hir = self.infer_expr(left, signatures)?;
                let right_hir = self.infer_expr(right, signatures)?;
                let left_type = self.apply(&left_hir.type_);
                let right_type = self.apply(&right_hir.type_);

                if let Err(e) = self.unify(&left_type, &right_type, *span) {
                    self.errors.push(e);
                }

                let result_type = match op {
                    BinaryOp::Eq
                    | BinaryOp::Ne
                    | BinaryOp::Lt
                    | BinaryOp::Gt
                    | BinaryOp::Le
                    | BinaryOp::Ge
                    | BinaryOp::And
                    | BinaryOp::Or => Type::Primitive(Primitive::Bool),
                    _ => self.apply(&left_hir.type_),
                };

                Ok(HirExpr {
                    kind: HirExprKind::Binary {
                        left: Box::new(left_hir),
                        op: op.clone(),
                        right: Box::new(right_hir),
                    },
                    type_: result_type,
                })
            }
            Expression::Unary { span, op, operand } => {
                let hir_operand = self.infer_expr(operand, signatures)?;
                let operand_type = self.apply(&hir_operand.type_);
                match op {
                    UnaryOp::Not => {
                        if let Err(e) =
                            self.unify(&operand_type, &Type::Primitive(Primitive::Bool), *span)
                        {
                            self.errors.push(e);
                        }
                        Ok(HirExpr {
                            kind: HirExprKind::Unary {
                                op: op.clone(),
                                operand: Box::new(hir_operand),
                            },
                            type_: Type::Primitive(Primitive::Bool),
                        })
                    }
                    UnaryOp::Neg => Ok(HirExpr {
                        kind: HirExprKind::Unary {
                            op: op.clone(),
                            operand: Box::new(hir_operand),
                        },
                        type_: operand_type,
                    }),
                    UnaryOp::Ref => {
                        // &expr — require mutable variable as target
                        let operand_type = self.apply(&hir_operand.type_);
                        Ok(HirExpr {
                            kind: HirExprKind::Ref(Box::new(hir_operand)),
                            type_: Type::Ref(Box::new(operand_type)),
                        })
                    }
                }
            }
            Expression::Call {
                span,
                function,
                args,
            } => {
                let hir_func = self.infer_expr(function, signatures)?;

                let hir_args: Result<Vec<HirExpr>, _> = args
                    .iter()
                    .map(|a| self.infer_expr(a, signatures))
                    .collect();
                let hir_args = hir_args?;

                if let HirExprKind::Ident(name) = &hir_func.kind
                    && let Some(sig) = signatures.get(name.as_str())
                {
                    if hir_args.len() != sig.params.len() {
                        self.errors.push(self.error(
                            *span,
                            format!(
                                "function `{name}` expects {} arguments, got {}",
                                sig.params.len(),
                                hir_args.len()
                            ),
                        ));
                    }

                    for (i, (arg, param)) in args.iter().zip(&sig.params).enumerate() {
                        let arg_type = self.apply(&hir_args[i].type_);
                        if matches!(&param.type_, Type::Ref(_))
                            && !matches!(arg, Expression::Ref { .. })
                        {
                            self.errors.push(self.error(
                                arg.span(),
                                format!(
                                    "argument {} to `{name}` must be a reference; use `&`",
                                    i + 1
                                ),
                            ));
                            continue;
                        }
                        if let Err(e) = self.unify(&arg_type, &param.type_, arg.span()) {
                            self.errors.push(e);
                        }
                        if let Type::Ref(_) = &param.type_
                            && let Expression::Ref { operand, .. } = arg
                            && let Expression::Ident(name, _) = operand.as_ref()
                            && let Some(scheme) = self.lookup(name)
                            && !scheme.mutable
                        {
                            self.errors.push(self.error(
                                arg.span(),
                                format!(
                                    "cannot pass immutable binding `{name}` as mutable reference"
                                ),
                            ));
                        }
                    }

                    return Ok(HirExpr {
                        kind: HirExprKind::Call {
                            function: Box::new(hir_func),
                            args: hir_args,
                        },
                        type_: sig
                            .return_type
                            .clone()
                            .unwrap_or(Type::Primitive(Primitive::Unit)),
                    });
                }

                self.errors
                    .push(self.error(*span, "cannot infer call target type".to_string()));
                Ok(HirExpr {
                    kind: HirExprKind::Call {
                        function: Box::new(hir_func),
                        args: hir_args,
                    },
                    type_: Type::Primitive(Primitive::Unit),
                })
            }
            Expression::Block(block, _) => {
                self.push_scope();
                let stmts = self.infer_block(block, signatures)?;
                self.pop_scope();
                Ok(HirExpr {
                    kind: HirExprKind::Block(stmts),
                    type_: Type::Primitive(Primitive::Unit),
                })
            }
            Expression::Array(elements, _) => {
                let mut hir_elements = Vec::new();
                let element_var = self.fresh_var();
                for element in elements {
                    let hir = self.infer_expr(element, signatures)?;
                    let resolved = self.apply(&hir.type_);
                    if let Err(e) = self.unify(&resolved, &element_var, element.span()) {
                        self.errors.push(e);
                    }
                    hir_elements.push(hir);
                }
                let element_type = self.apply(&element_var);
                Ok(HirExpr {
                    kind: HirExprKind::Array(hir_elements),
                    type_: Type::Array {
                        element: Box::new(element_type),
                        size: elements.len(),
                    },
                })
            }
            Expression::Index { array, index, span } => {
                let hir_array = self.infer_expr(array, signatures)?;
                let hir_index = self.infer_expr(index, signatures)?;
                let array_type = self.apply(&hir_array.type_);
                let element_type = match &array_type {
                    Type::Array { element, .. } => *element.clone(),
                    Type::Primitive(Primitive::String) => Type::Primitive(Primitive::Char),
                    _ => {
                        self.errors.push(
                            self.error(expr.span(), format!("cannot index type `{}`", array_type)),
                        );
                        self.fresh_var()
                    }
                };
                Ok(HirExpr {
                    kind: HirExprKind::Index {
                        span: *span,
                        array: Box::new(hir_array),
                        index: Box::new(hir_index),
                    },
                    type_: element_type,
                })
            }
            Expression::Paren(inner, _) => self.infer_expr(inner, signatures),
            Expression::Ref { span, operand } => {
                if matches!(operand.as_ref(), Expression::Index { .. }) {
                    return Err(self.error(
                        *span,
                        "cannot take reference to array index element".to_string(),
                    ));
                }
                let hir_operand = self.infer_expr(operand, signatures)?;
                let operand_type = self.apply(&hir_operand.type_);
                Ok(HirExpr {
                    kind: HirExprKind::Ref(Box::new(hir_operand)),
                    type_: Type::Ref(Box::new(operand_type)),
                })
            }
            Expression::If {
                condition,
                then_block,
                else_if,
                else_block,
                span,
            } => {
                let hir_condition = self.infer_expr(condition, signatures)?;
                let cond_type = self.apply(&hir_condition.type_);
                if let Err(e) = self.unify(
                    &cond_type,
                    &Type::Primitive(Primitive::Bool),
                    condition.span(),
                ) {
                    self.errors.push(e);
                }

                self.push_scope();
                let hir_then = self.infer_block(then_block, signatures)?;
                self.pop_scope();

                let mut hir_else_if = Vec::new();
                for (cond, block) in else_if {
                    let c = self.infer_expr(cond, signatures)?;
                    let c_type = self.apply(&c.type_);
                    if let Err(e) =
                        self.unify(&c_type, &Type::Primitive(Primitive::Bool), cond.span())
                    {
                        self.errors.push(e);
                    }
                    self.push_scope();
                    let b = self.infer_block(block, signatures)?;
                    self.pop_scope();
                    hir_else_if.push((c, b));
                }

                let hir_else = else_block
                    .as_ref()
                    .map(|block| {
                        self.push_scope();
                        let result = self.infer_block(block, signatures);
                        self.pop_scope();
                        result
                    })
                    .transpose()?;

                let result_type =
                    self.infer_if_result_type(&hir_then, &hir_else_if, &hir_else, *span)?;

                Ok(HirExpr {
                    kind: HirExprKind::If {
                        condition: Box::new(hir_condition),
                        then_block: hir_then,
                        else_if: hir_else_if,
                        else_block: hir_else,
                    },
                    type_: result_type,
                })
            }
            Expression::Tuple(elements, span) => {
                let mut hir_elements = Vec::new();
                let mut element_types = Vec::new();
                for element in elements {
                    let hir = self.infer_expr(element, signatures)?;
                    element_types.push(self.apply(&hir.type_));
                    hir_elements.push(hir);
                }
                Ok(HirExpr {
                    kind: HirExprKind::Tuple(hir_elements, *span),
                    type_: Type::Tuple(element_types),
                })
            }
            Expression::Field { span, object, name } => {
                let hir_object = self.infer_expr(object, signatures)?;
                let object_type = self.apply(&hir_object.type_);
                let field_type = self.resolve_field_type(&object_type, name, *span);
                Ok(HirExpr {
                    kind: HirExprKind::FieldAccess {
                        span: *span,
                        object: Box::new(hir_object),
                        name: name.clone(),
                    },
                    type_: field_type,
                })
            }
            Expression::EnumVariant {
                span,
                type_name,
                variant_name,
                args,
            } => {
                let variant_info = self.types.get(type_name).and_then(|kind| {
                    if let HirItemKind::Enum(e) = kind {
                        e.variants
                            .iter()
                            .position(|v| v.name == *variant_name)
                            .map(|idx| {
                                let variant = &e.variants[idx];
                                let expected: Vec<Type> = match &variant.data {
                                    Some(HirEnumVariantData::Tuple(types)) => types.clone(),
                                    Some(HirEnumVariantData::Struct(fields)) => {
                                        fields.iter().map(|f| f.type_.clone()).collect()
                                    }
                                    None => Vec::new(),
                                };
                                (idx, expected)
                            })
                    } else {
                        None
                    }
                });
                let (variant_index, expected_types) = match variant_info {
                    Some(info) => info,
                    None => {
                        return Err(self.error(
                            *span,
                            format!("enum `{type_name}` has no variant `{variant_name}`"),
                        ));
                    }
                };
                if args.len() != expected_types.len() {
                    self.errors.push(self.error(
                        *span,
                        format!(
                            "variant `{variant_name}` expects {} arguments, got {}",
                            expected_types.len(),
                            args.len()
                        ),
                    ));
                }
                let mut payload = Vec::new();
                for (i, arg) in args.iter().enumerate() {
                    let hir_arg = self.infer_expr(arg, signatures)?;
                    if let Some(expected) = expected_types.get(i) {
                        let arg_type = self.apply(&hir_arg.type_);
                        if let Err(e) = self.unify(&arg_type, expected, arg.span()) {
                            self.errors.push(e);
                        }
                    }
                    payload.push(hir_arg);
                }
                Ok(HirExpr {
                    kind: HirExprKind::EnumVariant {
                        type_name: type_name.clone(),
                        variant_index,
                        payload,
                    },
                    type_: Type::Named(type_name.clone()),
                })
            }
            Expression::Match { span, .. } => {
                return Err(self.error(*span, "match expressions not supported yet".to_string()));
            }
        }
    }

    fn resolve_field_type(
        &mut self,
        object_type: &Type,
        field_name: &str,
        span: SourceSpan,
    ) -> Type {
        match object_type {
            Type::Named(name) => {
                if let Some(HirItemKind::Struct(s)) = self.types.get(name) {
                    if let Some(field) = s.fields.iter().find(|f| f.name == field_name) {
                        return field.type_.clone();
                    }
                    self.errors.push(
                        self.error(span, format!("struct `{name}` has no field `{field_name}`")),
                    );
                    return self.fresh_var();
                }
                if let Some(HirItemKind::TupleStruct(t)) = self.types.get(name) {
                    if let Ok(index) = field_name.parse::<usize>() {
                        if index < t.types.len() {
                            return t.types[index].clone();
                        }
                    }
                    self.errors.push(self.error(
                        span,
                        format!("tuple struct `{name}` has no field `{field_name}`"),
                    ));
                    return self.fresh_var();
                }
                self.fresh_var()
            }
            Type::Tuple(elements) => {
                if let Ok(index) = field_name.parse::<usize>() {
                    if index < elements.len() {
                        return elements[index].clone();
                    }
                }
                self.errors
                    .push(self.error(span, format!("tuple index out of bounds: `{field_name}`")));
                self.fresh_var()
            }
            _ => self.fresh_var(),
        }
    }

    fn fresh_var(&mut self) -> Type {
        let id = self.next_var;
        self.next_var += 1;
        Type::Var(id)
    }

    fn infer_if_result_type(
        &mut self,
        then: &[HirStatement],
        else_if: &[(HirExpr, Vec<HirStatement>)],
        else_: &Option<Vec<HirStatement>>,
        span: SourceSpan,
    ) -> Result<Type, TypeError> {
        let then_type = self.block_result_type(then);
        let mut types = vec![then_type];
        for (_, block) in else_if {
            types.push(self.block_result_type(block));
        }
        match else_ {
            Some(block) => types.push(self.block_result_type(block)),
            None => return Ok(Type::Primitive(Primitive::Unit)),
        }

        let result = self.fresh_var();
        for t in &types {
            if let Err(e) = self.unify(&result, t, span) {
                self.errors.push(e);
            }
        }
        Ok(self.apply(&result))
    }

    fn block_result_type(&self, stmts: &[HirStatement]) -> Type {
        match stmts.last() {
            Some(HirStatement {
                kind: HirStatementKind::Value(expr),
                ..
            }) => expr.type_.clone(),
            Some(HirStatement {
                kind: HirStatementKind::Return(Some(expr)),
                ..
            }) => expr.type_.clone(),
            _ => Type::Primitive(Primitive::Unit),
        }
    }

    fn resolve_hir_type(&self, t: &Type) -> Type {
        match t {
            Type::Var(id) => {
                if let Some(resolved) = self.subs.get(id) {
                    self.resolve_hir_type(resolved)
                } else if self.float_vars.contains(id) {
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

    fn resolve_hir_expr(&self, expr: &HirExpr) -> HirExpr {
        HirExpr {
            kind: match &expr.kind {
                HirExprKind::Binary { left, op, right } => HirExprKind::Binary {
                    left: Box::new(self.resolve_hir_expr(left)),
                    op: op.clone(),
                    right: Box::new(self.resolve_hir_expr(right)),
                },
                HirExprKind::Unary { op, operand } => HirExprKind::Unary {
                    op: op.clone(),
                    operand: Box::new(self.resolve_hir_expr(operand)),
                },
                HirExprKind::Call { function, args } => HirExprKind::Call {
                    function: Box::new(self.resolve_hir_expr(function)),
                    args: args.iter().map(|a| self.resolve_hir_expr(a)).collect(),
                },
                HirExprKind::Block(stmts) => {
                    HirExprKind::Block(stmts.iter().map(|s| self.resolve_hir_stmt(s)).collect())
                }
                HirExprKind::Index { span, array, index } => HirExprKind::Index {
                    span: *span,
                    array: Box::new(self.resolve_hir_expr(array)),
                    index: Box::new(self.resolve_hir_expr(index)),
                },
                HirExprKind::Array(elements) => {
                    HirExprKind::Array(elements.iter().map(|e| self.resolve_hir_expr(e)).collect())
                }
                HirExprKind::If {
                    condition,
                    then_block,
                    else_if,
                    else_block,
                } => HirExprKind::If {
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
                HirExprKind::Unit => HirExprKind::Unit,
                HirExprKind::Ref(expr) => HirExprKind::Ref(Box::new(self.resolve_hir_expr(expr))),
                HirExprKind::Tuple(elements, span) => HirExprKind::Tuple(
                    elements.iter().map(|e| self.resolve_hir_expr(e)).collect(),
                    *span,
                ),
                HirExprKind::FieldAccess { span, object, name } => HirExprKind::FieldAccess {
                    span: *span,
                    object: Box::new(self.resolve_hir_expr(object)),
                    name: name.clone(),
                },
                HirExprKind::EnumVariant {
                    type_name,
                    variant_index,
                    payload,
                } => HirExprKind::EnumVariant {
                    type_name: type_name.clone(),
                    variant_index: *variant_index,
                    payload: payload.iter().map(|e| self.resolve_hir_expr(e)).collect(),
                },
                other => other.clone(),
            },
            type_: self.resolve_hir_type(&expr.type_),
        }
    }

    fn resolve_hir_stmt(&self, stmt: &HirStatement) -> HirStatement {
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

    fn resolve_hir_assign_target(&self, target: &HirAssignTarget) -> HirAssignTarget {
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

    fn resolve_hir_stmts(&self, stmts: Vec<HirStatement>) -> Vec<HirStatement> {
        stmts.iter().map(|s| self.resolve_hir_stmt(s)).collect()
    }

    fn validate_literal_types_stmt(&self, stmt: &HirStatement, errors: &mut Vec<TypeError>) {
        match &stmt.kind {
            HirStatementKind::Let { value, .. } => self.validate_literal_types_expr(value, errors),
            HirStatementKind::Expr(expr) => self.validate_literal_types_expr(expr, errors),
            HirStatementKind::Return(expr) => {
                if let Some(e) = expr {
                    self.validate_literal_types_expr(e, errors);
                }
            }
            HirStatementKind::Value(expr) => self.validate_literal_types_expr(expr, errors),
            HirStatementKind::Loop { body } => {
                self.validate_literal_types(body, errors);
            }
            HirStatementKind::Break | HirStatementKind::Continue => {}
            HirStatementKind::Assign { value, .. } => {
                self.validate_literal_types_expr(value, errors);
            }
        }
    }

    fn validate_literal_types_expr(&self, expr: &HirExpr, errors: &mut Vec<TypeError>) {
        match &expr.kind {
            HirExprKind::Int(_, span) if !expr.type_.is_numeric() => {
                errors.push(self.error(
                    *span,
                    format!(
                        "integer literal must be a numeric type, found `{}`",
                        expr.type_
                    ),
                ));
            }
            HirExprKind::Float(_, span) if !expr.type_.is_float() => {
                errors.push(self.error(
                    *span,
                    format!("float literal must be a float type, found `{}`", expr.type_),
                ));
            }
            HirExprKind::Binary { left, right, .. } => {
                self.validate_literal_types_expr(left, errors);
                self.validate_literal_types_expr(right, errors);
            }
            HirExprKind::Unary { operand, .. } => {
                self.validate_literal_types_expr(operand, errors);
            }
            HirExprKind::Call { function, args } => {
                self.validate_literal_types_expr(function, errors);
                for arg in args {
                    self.validate_literal_types_expr(arg, errors);
                }
            }
            HirExprKind::Block(stmts) => self.validate_literal_types(stmts, errors),
            HirExprKind::Index { span, array, index } => {
                self.validate_literal_types_expr(array, errors);
                self.validate_literal_types_expr(index, errors);
                if !index.type_.is_int() && !index.type_.is_uint() {
                    errors.push(self.error(
                        *span,
                        format!("index type must be an integer, found `{}`", index.type_),
                    ));
                }
            }
            HirExprKind::Array(elements) => {
                for e in elements {
                    self.validate_literal_types_expr(e, errors);
                }
            }
            HirExprKind::If {
                condition,
                then_block,
                else_if,
                else_block,
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

    fn collect_literal_type_errors(&self, stmts: &[HirStatement]) -> Vec<TypeError> {
        let mut errors = Vec::new();
        self.validate_literal_types(stmts, &mut errors);
        errors
    }

    fn hir_assign_op(op: &AssignOp) -> HirAssignOp {
        match op {
            AssignOp::Eq => HirAssignOp::Eq,
            AssignOp::AddEq => HirAssignOp::AddEq,
            AssignOp::SubEq => HirAssignOp::SubEq,
            AssignOp::MulEq => HirAssignOp::MulEq,
            AssignOp::DivEq => HirAssignOp::DivEq,
            AssignOp::RemEq => HirAssignOp::RemEq,
            AssignOp::BitAndEq => HirAssignOp::BitAndEq,
            AssignOp::BitOrEq => HirAssignOp::BitOrEq,
            AssignOp::BitXorEq => HirAssignOp::BitXorEq,
            AssignOp::ShlEq => HirAssignOp::ShlEq,
            AssignOp::ShrEq => HirAssignOp::ShrEq,
        }
    }

    fn validate_literal_types(&self, stmts: &[HirStatement], errors: &mut Vec<TypeError>) {
        for stmt in stmts {
            self.validate_literal_types_stmt(stmt, errors);
        }
    }
}
