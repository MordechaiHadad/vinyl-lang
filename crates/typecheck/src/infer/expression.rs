use std::collections::HashMap;

use miette::SourceSpan;
use vinyl_parser::ast::expression::Expression;
use vinyl_parser::ast::item::FunctionDef;
use vinyl_parser::ast::operator::{BinaryOp, UnaryOp};
use vinyl_parser::ast::types::Primitive;

use crate::error::TypeError;
use crate::hir::{
    HirEnumVariantData, HirExpression, HirExpressionKind, HirItemKind, HirStatement,
    HirStatementKind, Type,
};
use crate::infer::InferState;

impl InferState {
    pub(super) fn infer_expr(
        &mut self,
        expr: &Expression,
        signatures: &HashMap<&str, &FunctionDef>,
    ) -> Result<HirExpression, TypeError> {
        match expr {
            Expression::Int(v, span) => Ok(HirExpression {
                kind: HirExpressionKind::Int(*v, *span),
                type_: self.subs.fresh_var(),
            }),
            Expression::Float(v, span) => {
                let var = self.subs.fresh_var();
                if let Type::Var(id) = &var {
                    self.subs.float_vars.insert(*id);
                }
                Ok(HirExpression {
                    kind: HirExpressionKind::Float(*v, *span),
                    type_: var,
                })
            }
            Expression::String(s, _) => Ok(HirExpression {
                kind: HirExpressionKind::String(s.clone()),
                type_: Type::Primitive(Primitive::String),
            }),
            Expression::Unit(_) => Ok(HirExpression {
                kind: HirExpressionKind::Unit,
                type_: Type::Primitive(Primitive::Unit),
            }),
            Expression::Bool(b, _) => Ok(HirExpression {
                kind: HirExpressionKind::Bool(*b),
                type_: Type::Primitive(Primitive::Bool),
            }),
            Expression::Char(c, _) => Ok(HirExpression {
                kind: HirExpressionKind::Char(*c),
                type_: Type::Primitive(Primitive::Char),
            }),
            Expression::Ident(name, span) => {
                let scheme = self.scope.lookup(name).cloned();
                match scheme {
                    Some(scheme) => {
                        let resolved = self.subs.apply(&scheme.type_);
                        if let Type::Ref(inner) = &resolved {
                            Ok(HirExpression {
                                kind: HirExpressionKind::Ident(name.clone()),
                                type_: *inner.clone(),
                            })
                        } else {
                            Ok(HirExpression {
                                kind: HirExpressionKind::Ident(name.clone()),
                                type_: scheme.type_,
                            })
                        }
                    }
                    None if signatures.contains_key(name.as_str()) => Ok(HirExpression {
                        kind: HirExpressionKind::Ident(name.clone()),
                        type_: Type::Primitive(Primitive::Unit),
                    }),
                    None => Err(self
                        .source
                        .error(*span, format!("undefined variable `{name}`"))),
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
                let left_type = self.subs.apply(&left_hir.type_);
                let right_type = self.subs.apply(&right_hir.type_);

                if let Err(e) = self
                    .subs
                    .unify(&self.source, &left_type, &right_type, *span)
                {
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
                    _ => self.subs.apply(&left_hir.type_),
                };

                Ok(HirExpression {
                    kind: HirExpressionKind::Binary {
                        left: Box::new(left_hir),
                        op: op.clone(),
                        right: Box::new(right_hir),
                    },
                    type_: result_type,
                })
            }
            Expression::Unary { span, op, operand } => {
                let hir_operand = self.infer_expr(operand, signatures)?;
                let operand_type = self.subs.apply(&hir_operand.type_);
                match op {
                    UnaryOp::Not => {
                        if let Err(e) = self.subs.unify(
                            &self.source,
                            &operand_type,
                            &Type::Primitive(Primitive::Bool),
                            *span,
                        ) {
                            self.errors.push(e);
                        }
                        Ok(HirExpression {
                            kind: HirExpressionKind::Unary {
                                op: op.clone(),
                                operand: Box::new(hir_operand),
                            },
                            type_: Type::Primitive(Primitive::Bool),
                        })
                    }
                    UnaryOp::Neg => Ok(HirExpression {
                        kind: HirExpressionKind::Unary {
                            op: op.clone(),
                            operand: Box::new(hir_operand),
                        },
                        type_: operand_type,
                    }),
                    UnaryOp::Ref => {
                        let operand_type = self.subs.apply(&hir_operand.type_);
                        Ok(HirExpression {
                            kind: HirExpressionKind::Ref(Box::new(hir_operand)),
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

                let hir_args: Result<Vec<HirExpression>, _> = args
                    .iter()
                    .map(|a| self.infer_expr(a, signatures))
                    .collect();
                let hir_args = hir_args?;

                if let HirExpressionKind::Ident(name) = &hir_func.kind
                    && let Some(sig) = signatures.get(name.as_str())
                {
                    if hir_args.len() != sig.params.len() {
                        self.errors.push(self.source.error(
                            *span,
                            format!(
                                "function `{name}` expects {} arguments, got {}",
                                sig.params.len(),
                                hir_args.len()
                            ),
                        ));
                    }

                    for (i, (arg, param)) in args.iter().zip(&sig.params).enumerate() {
                        let arg_type = self.subs.apply(&hir_args[i].type_);
                        if matches!(&param.type_, Type::Ref(_))
                            && !matches!(arg, Expression::Ref { .. })
                        {
                            self.errors.push(self.source.error(
                                arg.span(),
                                format!(
                                    "argument {} to `{name}` must be a reference; use `&`",
                                    i + 1
                                ),
                            ));
                            continue;
                        }
                        if let Err(e) =
                            self.subs
                                .unify(&self.source, &arg_type, &param.type_, arg.span())
                        {
                            self.errors.push(e);
                        }
                        if let Type::Ref(_) = &param.type_
                            && let Expression::Ref { operand, .. } = arg
                            && let Expression::Ident(name, _) = operand.as_ref()
                            && let Some(scheme) = self.scope.lookup(name)
                            && !scheme.mutable
                        {
                            self.errors.push(self.source.error(
                                arg.span(),
                                format!(
                                    "cannot pass immutable binding `{name}` as mutable reference"
                                ),
                            ));
                        }
                    }

                    return Ok(HirExpression {
                        kind: HirExpressionKind::Call {
                            function: Box::new(hir_func),
                            args: hir_args,
                        },
                        type_: sig
                            .return_type
                            .clone()
                            .unwrap_or(Type::Primitive(Primitive::Unit)),
                    });
                }

                self.errors.push(
                    self.source
                        .error(*span, "cannot infer call target type".to_string()),
                );
                Ok(HirExpression {
                    kind: HirExpressionKind::Call {
                        function: Box::new(hir_func),
                        args: hir_args,
                    },
                    type_: Type::Primitive(Primitive::Unit),
                })
            }
            Expression::Block(block, _) => {
                self.scope.push_scope();
                let stmts = self.infer_block(block, signatures)?;
                self.scope.pop_scope();
                Ok(HirExpression {
                    kind: HirExpressionKind::Block(stmts),
                    type_: Type::Primitive(Primitive::Unit),
                })
            }
            Expression::Array(elements, _) => {
                let mut hir_elements = Vec::new();
                let element_var = self.subs.fresh_var();
                for element in elements {
                    let hir = self.infer_expr(element, signatures)?;
                    let resolved = self.subs.apply(&hir.type_);
                    if let Err(e) =
                        self.subs
                            .unify(&self.source, &resolved, &element_var, element.span())
                    {
                        self.errors.push(e);
                    }
                    hir_elements.push(hir);
                }
                let element_type = self.subs.apply(&element_var);
                Ok(HirExpression {
                    kind: HirExpressionKind::Array(hir_elements),
                    type_: Type::Array {
                        element: Box::new(element_type),
                        size: elements.len(),
                    },
                })
            }
            Expression::Index { array, index, span } => {
                let hir_array = self.infer_expr(array, signatures)?;
                let hir_index = self.infer_expr(index, signatures)?;
                let array_type = self.subs.apply(&hir_array.type_);
                let element_type = match &array_type {
                    Type::Array { element, .. } => *element.clone(),
                    Type::Primitive(Primitive::String) => Type::Primitive(Primitive::Char),
                    _ => {
                        self.errors.push(
                            self.source
                                .error(expr.span(), format!("cannot index type `{}`", array_type)),
                        );
                        self.subs.fresh_var()
                    }
                };
                Ok(HirExpression {
                    kind: HirExpressionKind::Index {
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
                    return Err(self.source.error(
                        *span,
                        "cannot take reference to array index element".to_string(),
                    ));
                }
                let hir_operand = self.infer_expr(operand, signatures)?;
                let operand_type = self.subs.apply(&hir_operand.type_);
                Ok(HirExpression {
                    kind: HirExpressionKind::Ref(Box::new(hir_operand)),
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
                let cond_type = self.subs.apply(&hir_condition.type_);
                if let Err(e) = self.subs.unify(
                    &self.source,
                    &cond_type,
                    &Type::Primitive(Primitive::Bool),
                    condition.span(),
                ) {
                    self.errors.push(e);
                }

                self.scope.push_scope();
                let hir_then = self.infer_block(then_block, signatures)?;
                self.scope.pop_scope();

                let mut hir_else_if = Vec::new();
                for (cond, block) in else_if {
                    let c = self.infer_expr(cond, signatures)?;
                    let c_type = self.subs.apply(&c.type_);
                    if let Err(e) = self.subs.unify(
                        &self.source,
                        &c_type,
                        &Type::Primitive(Primitive::Bool),
                        cond.span(),
                    ) {
                        self.errors.push(e);
                    }
                    self.scope.push_scope();
                    let b = self.infer_block(block, signatures)?;
                    self.scope.pop_scope();
                    hir_else_if.push((c, b));
                }

                let hir_else = else_block
                    .as_ref()
                    .map(|block| {
                        self.scope.push_scope();
                        let result = self.infer_block(block, signatures);
                        self.scope.pop_scope();
                        result
                    })
                    .transpose()?;

                let result_type =
                    self.infer_if_result_type(&hir_then, &hir_else_if, &hir_else, *span)?;

                Ok(HirExpression {
                    kind: HirExpressionKind::If {
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
                    element_types.push(self.subs.apply(&hir.type_));
                    hir_elements.push(hir);
                }
                Ok(HirExpression {
                    kind: HirExpressionKind::Tuple(hir_elements, *span),
                    type_: Type::Tuple(element_types),
                })
            }
            Expression::Field { span, object, name } => {
                let hir_object = self.infer_expr(object, signatures)?;
                let object_type = self.subs.apply(&hir_object.type_);
                let field_type = self.resolve_field_type(&object_type, name, *span);
                Ok(HirExpression {
                    kind: HirExpressionKind::FieldAccess {
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
                if let Some(function) = self
                    .module_table
                    .get(type_name)
                    .and_then(|module| module.functions.iter().find(|f| f.name == *variant_name))
                    .cloned()
                {
                    let mut hir_args = Vec::new();
                    for (index, arg) in args.iter().enumerate() {
                        let hir_arg = self.infer_expr(arg, signatures)?;
                        if let Some(param) = function.params.get(index) {
                            let arg_type = self.subs.apply(&hir_arg.type_);
                            if let Err(error) =
                                self.subs
                                    .unify(&self.source, &arg_type, &param.type_, arg.span())
                            {
                                self.errors.push(error);
                            }
                        }
                        hir_args.push(hir_arg);
                    }
                    if args.len() != function.params.len() {
                        self.errors.push(self.source.error(
                            *span,
                            format!(
                                "function `{type_name}::{variant_name}` expects {} arguments, got {}",
                                function.params.len(),
                                args.len()
                            ),
                        ));
                    }
                    return Ok(HirExpression {
                        kind: HirExpressionKind::Call {
                            function: Box::new(HirExpression {
                                kind: HirExpressionKind::Ident(format!(
                                    "{type_name}::{variant_name}"
                                )),
                                type_: Type::Primitive(Primitive::Unit),
                            }),
                            args: hir_args,
                        },
                        type_: function
                            .return_type
                            .clone()
                            .unwrap_or(Type::Primitive(Primitive::Unit)),
                    });
                }
                if self.module_table.contains_key(type_name) {
                    return Err(self.source.error(
                        *span,
                        format!("item `{type_name}::{variant_name}` is private or not found"),
                    ));
                }
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
                        return Err(self.source.error(
                            *span,
                            format!("enum `{type_name}` has no variant `{variant_name}`"),
                        ));
                    }
                };
                if args.len() != expected_types.len() {
                    self.errors.push(self.source.error(
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
                        let arg_type = self.subs.apply(&hir_arg.type_);
                        if let Err(e) =
                            self.subs
                                .unify(&self.source, &arg_type, expected, arg.span())
                        {
                            self.errors.push(e);
                        }
                    }
                    payload.push(hir_arg);
                }
                Ok(HirExpression {
                    kind: HirExpressionKind::EnumVariant {
                        type_name: type_name.clone(),
                        variant_index,
                        payload,
                    },
                    type_: Type::Named(type_name.clone()),
                })
            }
            Expression::Struct {
                span,
                type_name,
                fields,
            } => {
                let mut hir_fields = Vec::new();
                for (name, expr) in fields {
                    let hir = self.infer_expr(expr, signatures)?;
                    if let Some(HirItemKind::Struct(s)) = self.types.get(type_name) {
                        if let Some(field) = s.fields.iter().find(|f| f.name == *name) {
                            let field_type = self.subs.apply(&hir.type_);
                            if let Err(e) = self.subs.unify(
                                &self.source,
                                &field_type,
                                &field.type_,
                                expr.span(),
                            ) {
                                self.errors.push(e);
                            }
                        } else {
                            self.errors.push(self.source.error(
                                expr.span(),
                                format!("struct `{type_name}` has no field `{name}`"),
                            ));
                        }
                    }
                    hir_fields.push((name.clone(), hir));
                }
                let struct_type = match self.types.get(type_name) {
                    Some(HirItemKind::Struct(s)) => {
                        for field in &s.fields {
                            if !hir_fields.iter().any(|(n, _)| n == &field.name) {
                                self.errors.push(self.source.error(
                                    *span,
                                    format!(
                                        "struct `{type_name}` is missing field `{}`",
                                        field.name
                                    ),
                                ));
                            }
                        }
                        Type::Named(type_name.clone())
                    }
                    _ => {
                        return Err(self
                            .source
                            .error(*span, format!("`{type_name}` is not a struct")));
                    }
                };
                Ok(HirExpression {
                    kind: HirExpressionKind::Struct {
                        type_name: type_name.clone(),
                        fields: hir_fields,
                    },
                    type_: struct_type,
                })
            }
            Expression::Match { span, .. } => Err(self
                .source
                .error(*span, "match expressions not supported yet".to_string())),
        }
    }

    pub(super) fn resolve_field_type(
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
                        self.source
                            .error(span, format!("struct `{name}` has no field `{field_name}`")),
                    );
                    return self.subs.fresh_var();
                }
                if let Some(HirItemKind::TupleStruct(t)) = self.types.get(name) {
                    if let Ok(index) = field_name.parse::<usize>()
                        && index < t.types.len()
                    {
                        return t.types[index].clone();
                    }
                    self.errors.push(self.source.error(
                        span,
                        format!("tuple struct `{name}` has no field `{field_name}`"),
                    ));
                    return self.subs.fresh_var();
                }
                self.subs.fresh_var()
            }
            Type::Tuple(elements) => {
                if let Ok(index) = field_name.parse::<usize>()
                    && index < elements.len()
                {
                    return elements[index].clone();
                }
                self.errors.push(
                    self.source
                        .error(span, format!("tuple index out of bounds: `{field_name}`")),
                );
                self.subs.fresh_var()
            }
            _ => self.subs.fresh_var(),
        }
    }

    pub(super) fn infer_if_result_type(
        &mut self,
        then: &[HirStatement],
        else_if: &[(HirExpression, Vec<HirStatement>)],
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

        let result = self.subs.fresh_var();
        for t in &types {
            if let Err(e) = self.subs.unify(&self.source, &result, t, span) {
                self.errors.push(e);
            }
        }
        Ok(self.subs.apply(&result))
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
}
