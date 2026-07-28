use std::collections::HashMap;

use miette::SourceSpan;

use crate::error::{InferResult, TypeDiagnosticKind};
use crate::infer::SourceContext;
use crate::infer::TypeScheme;

pub(super) struct ScopeState {
    pub(super) scopes: Vec<HashMap<String, TypeScheme>>,
}

impl ScopeState {
    pub(super) fn new() -> Self {
        ScopeState {
            scopes: vec![HashMap::new()],
        }
    }

    pub(super) fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub(super) fn pop_scope(&mut self) {
        self.scopes.pop();
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
            TypeDiagnosticKind::UndefinedName {
                name: name.to_string(),
            },
        )))
    }
}
