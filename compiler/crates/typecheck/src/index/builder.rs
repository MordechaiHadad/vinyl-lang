use std::collections::{BTreeMap, HashMap};

use crate::hir::{
    HirExpression, HirExpressionKind, HirItem, HirItemKind, HirStatement, HirStatementKind,
};
use crate::index::types::{Definition, DefinitionKind, HirExprRef, HirIndex};

pub struct IndexBuilder {
    expr_at_pos: BTreeMap<usize, HirExprRef>,
    definitions: HashMap<String, Vec<Definition>>,
    references: BTreeMap<usize, Definition>,
    scopes: Vec<HashMap<String, usize>>,
    next_definition_id: usize,
}

impl Default for IndexBuilder {
    fn default() -> Self {
        Self {
            expr_at_pos: BTreeMap::new(),
            definitions: HashMap::new(),
            references: BTreeMap::new(),
            scopes: vec![HashMap::new()],
            next_definition_id: 0,
        }
    }
}

impl IndexBuilder {
    pub fn build(mut self, items: &[HirItem]) -> HirIndex {
        self.collect_global_definitions(items);
        for item in items {
            self.walk_item(item);
        }
        let referenced: std::collections::HashSet<usize> = self.references.values().map(|d| d.id).collect();
        let unused = self.definitions.values()
            .flat_map(|defs| defs.iter())
            .filter(|d| d.scope_depth > 1 && !referenced.contains(&d.id) && d.name != "main")
            .cloned()
            .collect();
        HirIndex {
            expr_at_pos: self.expr_at_pos,
            definitions: self.definitions,
            references: self.references,
            unused,
        }
    }

    fn collect_global_definitions(&mut self, items: &[HirItem]) {
        for item in items {
            let (name, kind, span) = match &item.kind {
                HirItemKind::Function(function) => {
                    (&function.name, DefinitionKind::Function, function.span)
                }
                HirItemKind::Struct(structure) => {
                    (&structure.name, DefinitionKind::Struct, structure.span)
                }
                HirItemKind::TupleStruct(tuple_struct) => (
                    &tuple_struct.name,
                    DefinitionKind::TupleStruct,
                    tuple_struct.span,
                ),
                HirItemKind::Enum(enumeration) => {
                    (&enumeration.name, DefinitionKind::Enum, enumeration.span)
                }
            };
            self.add_definition(name, kind, span, None);
        }
    }

    fn add_definition(&mut self, name: &str, kind: DefinitionKind, span: miette::SourceSpan, type_name: Option<String>) {
        let definition = Definition {
            id: self.next_definition_id,
            name: name.to_string(),
            kind,
            span,
            scope_depth: self.scopes.len(),
            type_name,
        };
        self.next_definition_id += 1;
        let id = definition.id;
        self.definitions
            .entry(name.to_string())
            .or_default()
            .push(definition);
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), id);
        }
    }

    fn definition_by_id(&self, id: usize) -> Option<Definition> {
        self.definitions
            .values()
            .flat_map(|definitions| definitions.iter())
            .find(|definition| definition.id == id)
            .cloned()
    }

    fn resolve_reference(&mut self, name: &str, span: miette::SourceSpan) {
        let id = self
            .scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
            .or_else(|| {
                self.definitions
                    .get(name)
                    .and_then(|definitions| definitions.first().map(|definition| definition.id))
            });
        if let Some(id) = id
            && let Some(definition) = self.definition_by_id(id)
        {
            self.references.insert(span.offset(), definition);
        }
    }

    fn walk_item(&mut self, item: &HirItem) {
        match &item.kind {
            HirItemKind::Function(f) => {
                self.scopes.push(HashMap::new());
                for param in &f.params {
                    self.add_definition(&param.name, DefinitionKind::Parameter, param.span, Some(param.type_.to_string()));
                }
                self.walk_stmts(&f.body);
                self.scopes.pop();
            }
            HirItemKind::Struct(_) | HirItemKind::TupleStruct(_) | HirItemKind::Enum(_) => {}
        }
    }

    fn walk_stmts(&mut self, stmts: &[HirStatement]) {
        for stmt in stmts {
            self.walk_stmt(stmt);
        }
    }

    fn walk_stmt(&mut self, stmt: &HirStatement) {
        match &stmt.kind {
            HirStatementKind::Let {
                span, name, value, type_, ..
            } => {
                self.walk_expr(value);
                self.add_definition(name, DefinitionKind::Variable, *span, Some(type_.to_string()));
            }
            HirStatementKind::Expr(expr, _) => self.walk_expr(expr),
            HirStatementKind::Return(expr, _) => {
                if let Some(e) = expr {
                    self.walk_expr(e);
                }
            }
            HirStatementKind::Value(expr, _) => self.walk_expr(expr),
            HirStatementKind::Loop { body, .. } => {
                self.scopes.push(HashMap::new());
                self.walk_stmts(body);
                self.scopes.pop();
            }
            HirStatementKind::Break(_) | HirStatementKind::Continue(_) => {}
            HirStatementKind::Assign { target, value, .. } => {
                self.walk_assign_target(target);
                self.walk_expr(value);
            }
        }
    }

    fn walk_assign_target(&mut self, target: &crate::hir::HirAssignTarget) {
        match target {
            crate::hir::HirAssignTarget::Ident(name, span) => self.resolve_reference(name, *span),
            crate::hir::HirAssignTarget::Index { array, index, .. } => {
                self.walk_expr(array);
                self.walk_expr(index);
            }
            crate::hir::HirAssignTarget::Field { object, .. } => self.walk_expr(object),
            crate::hir::HirAssignTarget::Deref(expr, _) => self.walk_expr(expr),
        }
    }

    fn walk_expr(&mut self, expr: &HirExpression) {
        self.insert_expr(expr);
        match &expr.kind {
            HirExpressionKind::Binary { left, right, .. } => {
                self.walk_expr(left);
                self.walk_expr(right);
            }
            HirExpressionKind::Unary { operand, .. } => self.walk_expr(operand),
            HirExpressionKind::Call { function, args, .. } => {
                self.walk_expr(function);
                for arg in args {
                    self.walk_expr(arg);
                }
            }
            HirExpressionKind::Block(stmts, _) => {
                self.scopes.push(HashMap::new());
                for stmt in stmts {
                    self.walk_stmt(stmt);
                }
                self.scopes.pop();
            }
            HirExpressionKind::Index { array, index, .. } => {
                self.walk_expr(array);
                self.walk_expr(index);
            }
            HirExpressionKind::Array(elements, _) => {
                for e in elements {
                    self.walk_expr(e);
                }
            }
            HirExpressionKind::If {
                condition,
                then_block,
                else_if,
                else_block,
                ..
            } => {
                self.walk_expr(condition);
                self.scopes.push(HashMap::new());
                for stmt in then_block {
                    self.walk_stmt(stmt);
                }
                self.scopes.pop();
                for (c, b) in else_if {
                    self.walk_expr(c);
                    self.scopes.push(HashMap::new());
                    for stmt in b {
                        self.walk_stmt(stmt);
                    }
                    self.scopes.pop();
                }
                if let Some(b) = else_block {
                    self.scopes.push(HashMap::new());
                    for stmt in b {
                        self.walk_stmt(stmt);
                    }
                    self.scopes.pop();
                }
            }
            HirExpressionKind::Ref(inner, _) => self.walk_expr(inner),
            HirExpressionKind::Tuple(elements, _) => {
                for e in elements {
                    self.walk_expr(e);
                }
            }
            HirExpressionKind::FieldAccess { object, .. } => self.walk_expr(object),
            HirExpressionKind::EnumVariant { payload, .. } => {
                for e in payload {
                    self.walk_expr(e);
                }
            }
            HirExpressionKind::Struct { fields, .. } => {
                for (_, e) in fields {
                    self.walk_expr(e);
                }
            }
            HirExpressionKind::Int(_, _)
            | HirExpressionKind::Float(_, _)
            | HirExpressionKind::String(_, _)
            | HirExpressionKind::Bool(_, _)
            | HirExpressionKind::Unit(_)
            | HirExpressionKind::Char(_, _) => {}
            HirExpressionKind::Ident(name, span) => self.resolve_reference(name, *span),
        }
    }

    fn insert_expr(&mut self, expr: &HirExpression) {
        let span = match &expr.kind {
            HirExpressionKind::Int(_, span)
            | HirExpressionKind::Float(_, span)
            | HirExpressionKind::String(_, span)
            | HirExpressionKind::Bool(_, span)
            | HirExpressionKind::Unit(span)
            | HirExpressionKind::Char(_, span)
            | HirExpressionKind::Ident(_, span)
            | HirExpressionKind::Block(_, span)
            | HirExpressionKind::Array(_, span)
            | HirExpressionKind::Ref(_, span)
            | HirExpressionKind::Tuple(_, span) => *span,
            HirExpressionKind::Binary { span, .. }
            | HirExpressionKind::Unary { span, .. }
            | HirExpressionKind::Call { span, .. }
            | HirExpressionKind::Index { span, .. }
            | HirExpressionKind::FieldAccess { span, .. }
            | HirExpressionKind::EnumVariant { span, .. }
            | HirExpressionKind::Struct { span, .. }
            | HirExpressionKind::If { span, .. } => *span,
        };
        self.expr_at_pos.insert(
            span.offset(),
            HirExprRef {
                span,
                kind: expr.kind.clone(),
                type_: expr.type_.clone(),
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::{
        HirExpression, HirExpressionKind, HirFunction, HirItem, HirItemKind, HirParam,
        HirStatement, HirStatementKind, Type,
    };
    use miette::SourceSpan;
    use vinyl_parser::ast::types::Primitive;

    #[test]
    fn resolves_shadowed_bindings_to_the_nearest_scope() {
        let outer_span = SourceSpan::from(0..1);
        let inner_span = SourceSpan::from(2..3);
        let reference_span = SourceSpan::from(4..5);
        let ident = |span| HirExpression {
            kind: HirExpressionKind::Ident("value".to_string(), span),
            type_: Type::Primitive(Primitive::Int32),
        };
        let items = vec![HirItem {
            span: SourceSpan::from(0..10),
            kind: HirItemKind::Function(HirFunction {
                span: SourceSpan::from(0..10),
                name: "main".to_string(),
                params: vec![HirParam {
                    span: SourceSpan::from(0..1),
                    name: "value".to_string(),
                    mutable: false,
                    type_: Type::Primitive(Primitive::Int32),
                }],
                return_type: Type::Primitive(Primitive::Unit),
                body: vec![HirStatement {
                    kind: HirStatementKind::Expr(
                        HirExpression {
                            kind: HirExpressionKind::Block(
                                vec![
                                    HirStatement {
                                        kind: HirStatementKind::Let {
                                            span: inner_span,
                                            name: "value".to_string(),
                                            mutable: false,
                                            type_: Type::Primitive(Primitive::Int32),
                                            value: ident(outer_span),
                                        },
                                    },
                                    HirStatement {
                                        kind: HirStatementKind::Value(
                                            ident(reference_span),
                                            reference_span,
                                        ),
                                    },
                                ],
                                SourceSpan::from(2..8),
                            ),
                            type_: Type::Primitive(Primitive::Unit),
                        },
                        SourceSpan::from(2..8),
                    ),
                }],
            }),
        }];
        let index = IndexBuilder::default().build(&items);
        let inner_definition = index.definitions["value"]
            .iter()
            .find(|definition| definition.span == inner_span)
            .unwrap();
        assert_eq!(
            index.references[&reference_span.offset()].id,
            inner_definition.id
        );
    }
}
