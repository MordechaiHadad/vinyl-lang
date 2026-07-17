use miette::{Diagnostic, NamedSource, SourceSpan};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use vinyl_parser::ast::{BinaryOp, Expr, FunctionDef, Item, Primitive, Stmt};

use crate::hir::{
    HirExpr, HirExprKind, HirFunction, HirItem, HirItemKind, HirParam, HirStmt, HirStmtKind, Type,
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

pub fn typeck(
    items: &[Item],
    source: &str,
    source_name: &str,
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

    let mut hir_items = Vec::new();
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

    if state.errors.is_empty() {
        Ok(hir_items)
    } else {
        Err(state.errors)
    }
}

#[derive(Debug, Clone)]
struct TypeScheme {
    type_: Type,
}

struct InferState {
    source: String,
    source_name: String,
    scopes: Vec<HashMap<String, TypeScheme>>,
    errors: Vec<TypeError>,
    subs: HashMap<usize, Type>,
    next_var: usize,
    current_return_type: Option<Type>,
}

impl InferState {
    fn new(source: &str, source_name: &str) -> Self {
        InferState {
            source: source.to_string(),
            source_name: source_name.to_string(),
            scopes: vec![HashMap::new()],
            errors: Vec::new(),
            subs: HashMap::new(),
            next_var: 0,
            current_return_type: None,
        }
    }

    fn fresh_var(&mut self) -> Type {
        let id = self.next_var;
        self.next_var += 1;
        Type::Var(id)
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
            other => other.clone(),
        }
    }

    fn occurs(&self, id: usize, t: &Type) -> bool {
        match t {
            Type::Var(vid) => *vid == id || self.subs.get(vid).is_some_and(|t| self.occurs(id, t)),
            Type::Ref(inner) => self.occurs(id, inner),
            Type::Array { element, .. } => self.occurs(id, element),
            Type::Generic { args, .. } => args.iter().any(|a| self.occurs(id, a)),
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
            _ => Err(self.error(span, format!("expected type `{}`, got `{}`", a, b))),
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

    fn error(&self, span: SourceSpan, message: String) -> TypeError {
        TypeError {
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
            params.push(HirParam {
                name: param.name.clone(),
                mutable: param.mutable,
                type_: param.type_.clone(),
            });
            self.bind(
                &param.name,
                TypeScheme {
                    type_: param.type_.clone(),
                },
            );
        }

        let return_type = match &func.return_type {
            Some(t) => t.clone(),
            None => self.fresh_var(),
        };

        let prev_return = self.current_return_type.replace(return_type.clone());
        self.push_scope();
        let body = self.infer_block(&func.body, signatures)?;
        self.pop_scope();
        self.current_return_type = prev_return;

        let body = self.resolve_hir_stmts(body);
        let mut return_type = self.apply(&return_type);
        if matches!(return_type, Type::Var(_)) {
            return_type = Type::Primitive(Primitive::Unit);
        }

        Ok(HirFunction {
            name: func.name.clone(),
            params,
            return_type,
            body,
        })
    }

    fn infer_block(
        &mut self,
        stmts: &[Stmt],
        signatures: &HashMap<&str, &FunctionDef>,
    ) -> Result<Vec<HirStmt>, TypeError> {
        let mut hir_stmts = Vec::new();
        for stmt in stmts {
            hir_stmts.push(self.infer_stmt(stmt, signatures)?);
        }
        Ok(hir_stmts)
    }

    fn infer_stmt(
        &mut self,
        stmt: &Stmt,
        signatures: &HashMap<&str, &FunctionDef>,
    ) -> Result<HirStmt, TypeError> {
        match stmt {
            Stmt::Let {
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
                };
                self.bind(name, scheme);

                Ok(HirStmt {
                    kind: HirStmtKind::Let {
                        name: name.clone(),
                        mutable: *mutable,
                        type_: value_type,
                        value: hir_value,
                    },
                })
            }
            Stmt::Expr(expr) => {
                let hir_expr = self.infer_expr(expr, signatures)?;
                Ok(HirStmt {
                    kind: HirStmtKind::Expr(hir_expr),
                })
            }
            Stmt::Return(expr, span) => {
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

                Ok(HirStmt {
                    kind: HirStmtKind::Return(hir_expr),
                })
            }
            Stmt::If {
                condition,
                then_block,
                else_if,
                else_block,
                ..
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

                Ok(HirStmt {
                    kind: HirStmtKind::If {
                        condition: hir_condition,
                        then_block: hir_then,
                        else_if: hir_else_if,
                        else_block: hir_else,
                    },
                })
            }
        }
    }

    fn infer_expr(
        &mut self,
        expr: &Expr,
        signatures: &HashMap<&str, &FunctionDef>,
    ) -> Result<HirExpr, TypeError> {
        match expr {
            Expr::Int(v, _) => Ok(HirExpr {
                kind: HirExprKind::Int(*v),
                type_: Type::Primitive(Primitive::Int32),
            }),
            Expr::Float(v, _) => Ok(HirExpr {
                kind: HirExprKind::Float(*v),
                type_: Type::Primitive(Primitive::Float64),
            }),
            Expr::String(s, _) => Ok(HirExpr {
                kind: HirExprKind::String(s.clone()),
                type_: Type::Primitive(Primitive::String),
            }),
            Expr::Bool(b, _) => Ok(HirExpr {
                kind: HirExprKind::Bool(*b),
                type_: Type::Primitive(Primitive::Bool),
            }),
            Expr::Ident(name, span) => {
                let scheme = self.lookup(name).cloned();
                match scheme {
                    Some(scheme) => Ok(HirExpr {
                        kind: HirExprKind::Ident(name.clone()),
                        type_: scheme.type_,
                    }),
                    None if signatures.contains_key(name.as_str()) => Ok(HirExpr {
                        kind: HirExprKind::Ident(name.clone()),
                        type_: Type::Primitive(Primitive::Unit),
                    }),
                    None => Err(self.error(*span, format!("undefined variable `{name}`"))),
                }
            }
            Expr::Binary {
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
            Expr::Call {
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
                        if let Err(e) = self.unify(&arg_type, &param.type_, arg.span()) {
                            self.errors.push(e);
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
            Expr::Block(block, _) => {
                self.push_scope();
                let stmts = self.infer_block(block, signatures)?;
                self.pop_scope();
                Ok(HirExpr {
                    kind: HirExprKind::Block(stmts),
                    type_: Type::Primitive(Primitive::Unit),
                })
            }
            Expr::Paren(inner, _) => self.infer_expr(inner, signatures),
            expr => Err(self.error(expr.span(), format!("unsupported expression: `{:?}`", expr))),
        }
    }

    fn resolve_hir_type(&self, t: &Type) -> Type {
        match t {
            Type::Var(id) => {
                if let Some(resolved) = self.subs.get(id) {
                    self.resolve_hir_type(resolved)
                } else {
                    Type::Primitive(Primitive::Unit)
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
                HirExprKind::Call { function, args } => HirExprKind::Call {
                    function: Box::new(self.resolve_hir_expr(function)),
                    args: args.iter().map(|a| self.resolve_hir_expr(a)).collect(),
                },
                HirExprKind::Block(stmts) => {
                    HirExprKind::Block(stmts.iter().map(|s| self.resolve_hir_stmt(s)).collect())
                }
                other => other.clone(),
            },
            type_: self.resolve_hir_type(&expr.type_),
        }
    }

    fn resolve_hir_stmt(&self, stmt: &HirStmt) -> HirStmt {
        HirStmt {
            kind: match &stmt.kind {
                HirStmtKind::Let {
                    name,
                    mutable,
                    type_,
                    value,
                } => HirStmtKind::Let {
                    name: name.clone(),
                    mutable: *mutable,
                    type_: self.resolve_hir_type(type_),
                    value: self.resolve_hir_expr(value),
                },
                HirStmtKind::Expr(expr) => HirStmtKind::Expr(self.resolve_hir_expr(expr)),
                HirStmtKind::Return(expr) => {
                    HirStmtKind::Return(expr.as_ref().map(|e| self.resolve_hir_expr(e)))
                }
                HirStmtKind::If {
                    condition,
                    then_block,
                    else_if,
                    else_block,
                } => HirStmtKind::If {
                    condition: self.resolve_hir_expr(condition),
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
            },
        }
    }

    fn resolve_hir_stmts(&self, stmts: Vec<HirStmt>) -> Vec<HirStmt> {
        stmts.iter().map(|s| self.resolve_hir_stmt(s)).collect()
    }
}
