use std::collections::{HashMap, HashSet};

use miette::SourceSpan;

use crate::error::TypeError;
use crate::hir::types::Type;
use crate::infer::SourceContext;

pub(super) struct SubstitutionState {
    pub(super) subs: HashMap<usize, Type>,
    pub(super) next_var: usize,
    pub(super) float_vars: HashSet<usize>,
}

impl SubstitutionState {
    pub(super) fn new() -> Self {
        SubstitutionState {
            subs: HashMap::new(),
            next_var: 0,
            float_vars: HashSet::new(),
        }
    }

    pub(super) fn resolve(&self, t: &Type) -> Type {
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

    pub(super) fn apply(&self, t: &Type) -> Type {
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

    pub(super) fn unify(
        &mut self,
        source: &SourceContext,
        a: &Type,
        b: &Type,
        span: SourceSpan,
    ) -> Result<(), TypeError> {
        let a = self.resolve(a);
        let b = self.resolve(b);

        if a == b {
            return Ok(());
        }

        match (&a, &b) {
            (Type::Var(id_a), _) => {
                if self.occurs(*id_a, &b) {
                    return Err(source.error(span, format!("recursive type: `{a}` contains `{b}`")));
                }
                self.subs.insert(*id_a, b.clone());
                Ok(())
            }
            (_, Type::Var(id_b)) => {
                if self.occurs(*id_b, &a) {
                    return Err(source.error(span, format!("recursive type: `{b}` contains `{a}`")));
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
                    self.unify(source, ai, bi, span)?;
                }
                Ok(())
            }
            (Type::Ref(t1), Type::Ref(t2)) => self.unify(source, t1, t2, span),
            (
                Type::Array {
                    element: e1,
                    size: s1,
                },
                Type::Array {
                    element: e2,
                    size: s2,
                },
            ) if s1 == s2 => self.unify(source, e1, e2, span),
            (Type::Tuple(a), Type::Tuple(b)) if a.len() == b.len() => {
                for (ai, bi) in a.iter().zip(b) {
                    self.unify(source, ai, bi, span)?;
                }
                Ok(())
            }
            _ => Err(source.type_mismatch(span, b, a)),
        }
    }

    pub(super) fn fresh_var(&mut self) -> Type {
        let id = self.next_var;
        self.next_var += 1;
        Type::Var(id)
    }
}
