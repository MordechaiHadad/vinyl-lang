use std::collections::HashMap;

use miette::SourceSpan;

use crate::error::{InferResult, TypeDiagnosticKind};
use crate::infer::SourceContext;
use crate::infer::TypeScheme;
use vinyl_parser::ast::item::FunctionDef;

pub(super) struct ScopeState {
    pub(super) scopes: Vec<HashMap<String, TypeScheme>>,
    pub(super) imports: Vec<HashMap<String, FunctionDef>>,
}

impl ScopeState {
    pub(super) fn new() -> Self {
        ScopeState {
            scopes: vec![HashMap::new()],
            imports: vec![HashMap::new()],
        }
    }

    pub(super) fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
        self.imports.push(HashMap::new());
    }

    pub(super) fn pop_scope(&mut self) {
        self.scopes.pop();
        self.imports.pop();
    }

    pub(super) fn bind(&mut self, name: &str, scheme: TypeScheme) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), scheme);
        }
    }

    pub(super) fn lookup(&self, name: &str) -> Option<&TypeScheme> {
        for scope in self.scopes.iter().rev() {
            if let Some(scheme) = scope.get(name) {
                return Some(scheme);
            }
        }
        None
    }

    pub(super) fn bind_import(&mut self, name: &str, function: FunctionDef) {
        if let Some(scope) = self.imports.last_mut() {
            scope.insert(name.to_string(), function);
        }
    }

    pub(super) fn lookup_import(&self, name: &str) -> Option<&FunctionDef> {
        for scope in self.imports.iter().rev() {
            if let Some(function) = scope.get(name) {
                return Some(function);
            }
        }
        None
    }

    pub(super) fn lookup_scope_index(&self, name: &str) -> Option<usize> {
        for (depth, scope) in self.scopes.iter().enumerate().rev() {
            if scope.contains_key(name) {
                return Some(depth);
            }
        }
        None
    }

    pub(super) fn check_assign_mutability(
        &self,
        source: &SourceContext,
        name: &str,
        span: SourceSpan,
    ) -> InferResult<()> {
        for scope in self.scopes.iter().rev() {
            if let Some(scheme) = scope.get(name) {
                if !scheme.mutable {
                    return Err(Box::new(source.error(
                        span,
                        TypeDiagnosticKind::AssignToImmutable {
                            name: name.to_string(),
                        },
                    )));
                }
                return Ok(());
            }
        }
        Err(Box::new(source.error(
            span,
            TypeDiagnosticKind::UndefinedVariable {
                name: name.to_string(),
            },
        )))
    }
}
