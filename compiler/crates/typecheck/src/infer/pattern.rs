use std::collections::HashMap;

use miette::SourceSpan;
use vinyl_parser::ast::pattern::{LiteralPattern, Pattern};
use vinyl_parser::ast::types::Primitive;

use crate::error::{InferResult, TypeDiagnosticKind};
use crate::hir::{
    HirEnumVariantData, HirItemKind, HirMatchArm, HirPattern, HirPatternKind, LiteralValue, Type,
};
use crate::infer::InferState;
use crate::infer::resolve_named_type;

impl InferState {
    pub(super) fn infer_pattern(
        &mut self,
        pattern: &Pattern,
        expected_type: &Type,
    ) -> InferResult<HirPattern> {
        let expected = self.subs.apply(expected_type);
        match pattern {
            Pattern::Wildcard(_) => Ok(HirPattern {
                kind: HirPatternKind::Wildcard(pattern.span()),
                type_: expected,
            }),
            Pattern::Ident(name, ident_span) => {
                self.scope.bind(
                    name,
                    crate::infer::TypeScheme {
                        type_: expected.clone(),
                        mutable: false,
                    },
                );
                Ok(HirPattern {
                    kind: HirPatternKind::Ident {
                        span: *ident_span,
                        name: name.clone(),
                    },
                    type_: expected,
                })
            }
            Pattern::Literal(lit, _) => {
                let value = match lit {
                    LiteralPattern::Int(v) => {
                        let var = self.subs.fresh_var();
                        if let Err(e) =
                            self.subs
                                .unify(&self.source, &var, &expected, pattern.span())
                        {
                            self.errors.push(*e);
                        }
                        LiteralValue::Int(*v)
                    }
                    LiteralPattern::Bool(b) => {
                        if let Err(e) = self.subs.unify(
                            &self.source,
                            &Type::Primitive(Primitive::Bool),
                            &expected,
                            pattern.span(),
                        ) {
                            self.errors.push(*e);
                        }
                        LiteralValue::Bool(*b)
                    }
                    LiteralPattern::Char(c) => {
                        if let Err(e) = self.subs.unify(
                            &self.source,
                            &Type::Primitive(Primitive::Char),
                            &expected,
                            pattern.span(),
                        ) {
                            self.errors.push(*e);
                        }
                        LiteralValue::Char(*c)
                    }
                    LiteralPattern::String(s) => {
                        if let Err(e) = self.subs.unify(
                            &self.source,
                            &Type::Primitive(Primitive::String),
                            &expected,
                            pattern.span(),
                        ) {
                            self.errors.push(*e);
                        }
                        LiteralValue::String(s.clone())
                    }
                };
                Ok(HirPattern {
                    kind: HirPatternKind::Literal {
                        span: pattern.span(),
                        value,
                    },
                    type_: expected,
                })
            }
            Pattern::Tuple(patterns, _) => {
                let tuple_type =
                    Type::Tuple((0..patterns.len()).map(|_| self.subs.fresh_var()).collect());
                if let Err(e) =
                    self.subs
                        .unify(&self.source, &tuple_type, &expected, pattern.span())
                {
                    self.errors.push(*e);
                }
                let mut hir_patterns = Vec::new();
                for element in patterns {
                    let element_type = match &expected {
                        Type::Tuple(elements) => elements
                            .get(hir_patterns.len())
                            .cloned()
                            .unwrap_or_else(|| self.subs.fresh_var()),
                        _ => self.subs.fresh_var(),
                    };
                    hir_patterns.push(self.infer_pattern(element, &element_type)?);
                }
                Ok(HirPattern {
                    kind: HirPatternKind::Tuple {
                        span: pattern.span(),
                        elements: hir_patterns,
                    },
                    type_: expected,
                })
            }
            Pattern::EnumVariant {
                span,
                type_path,
                variant_name,
                patterns,
            } => {
                let canonical_type_name = self.canonicalize_scoped_name(type_path, *span)?;
                if let Err(e) = self.subs.unify(
                    &self.source,
                    &Type::Named(canonical_type_name.clone()),
                    &expected,
                    *span,
                ) {
                    self.errors.push(*e);
                }
                let variant_info =
                    resolve_named_type(&canonical_type_name, &self.types).and_then(|kind| {
                        if let HirItemKind::Enum(e) = kind {
                            e.variants
                                .iter()
                                .position(|v| v.name == *variant_name)
                                .map(|idx| {
                                    let variant = &e.variants[idx];
                                    let expected_types = match &variant.data {
                                        Some(HirEnumVariantData::Tuple(types)) => types.clone(),
                                        Some(HirEnumVariantData::Struct(fields)) => {
                                            fields.iter().map(|f| f.type_.clone()).collect()
                                        }
                                        None => Vec::new(),
                                    };
                                    (idx, expected_types)
                                })
                        } else {
                            None
                        }
                    });
                let (variant_index, expected_types) = match variant_info {
                    Some(info) => info,
                    None if !self.types.contains_key(&canonical_type_name) => {
                        return Err(Box::new(self.source.error(
                            *span,
                            TypeDiagnosticKind::UndefinedType {
                                name: canonical_type_name.clone(),
                            },
                        )));
                    }
                    None => {
                        return Err(Box::new(self.source.error(
                            *span,
                            TypeDiagnosticKind::VariantNotFound {
                                type_name: canonical_type_name.clone(),
                                variant_name: variant_name.clone(),
                            },
                        )));
                    }
                };
                if patterns.len() != expected_types.len() {
                    self.errors.push(self.source.error(
                        *span,
                        TypeDiagnosticKind::VariantArgCountMismatch {
                            type_name: type_path.clone(),
                            variant_name: variant_name.clone(),
                            expected: expected_types.len(),
                            found: patterns.len(),
                        },
                    ));
                }
                let mut hir_patterns = Vec::new();
                for (index, sub_pattern) in patterns.iter().enumerate() {
                    let sub_type = expected_types
                        .get(index)
                        .cloned()
                        .unwrap_or_else(|| self.subs.fresh_var());
                    hir_patterns.push(self.infer_pattern(sub_pattern, &sub_type)?);
                }
                Ok(HirPattern {
                    kind: HirPatternKind::EnumVariant {
                        span: *span,
                        type_name: canonical_type_name,
                        variant_index,
                        patterns: hir_patterns,
                    },
                    type_: expected,
                })
            }
            Pattern::Struct { span, name, fields } => {
                let canonical_type_name = self.canonicalize_scoped_name(name, *span)?;
                if let Err(e) = self.subs.unify(
                    &self.source,
                    &Type::Named(canonical_type_name.clone()),
                    &expected,
                    *span,
                ) {
                    self.errors.push(*e);
                }
                let mut field_types: HashMap<String, Type> = HashMap::new();
                match resolve_named_type(&canonical_type_name, &self.types) {
                    Some(HirItemKind::Struct(s)) => {
                        for field in &s.fields {
                            if self.type_origins.contains_key(&canonical_type_name) && !field.public
                            {
                                self.errors.push(self.source.error(
                                    pattern.span(),
                                    TypeDiagnosticKind::PrivateField {
                                        type_name: canonical_type_name.clone(),
                                        field_name: field.name.clone(),
                                    },
                                ));
                            }
                            field_types.insert(field.name.clone(), field.type_.clone());
                        }
                    }
                    Some(HirItemKind::TupleStruct(t)) => {
                        for (index, type_) in t.types.iter().enumerate() {
                            field_types.insert(index.to_string(), type_.clone());
                        }
                    }
                    _ if !self.types.contains_key(&canonical_type_name) => {
                        self.errors.push(self.source.error(
                            *span,
                            TypeDiagnosticKind::UndefinedType {
                                name: canonical_type_name.clone(),
                            },
                        ));
                    }
                    _ => {
                        self.errors.push(self.source.error(
                            *span,
                            TypeDiagnosticKind::NotAStruct {
                                name: canonical_type_name.clone(),
                            },
                        ));
                    }
                }
                let mut hir_fields = Vec::new();
                for (field_name, sub_pattern) in fields {
                    let field_type = field_types
                        .get(field_name)
                        .cloned()
                        .unwrap_or_else(|| self.subs.fresh_var());
                    hir_fields.push((
                        field_name.clone(),
                        self.infer_pattern(sub_pattern, &field_type)?,
                    ));
                }
                Ok(HirPattern {
                    kind: HirPatternKind::Struct {
                        span: *span,
                        type_name: canonical_type_name,
                        fields: hir_fields,
                    },
                    type_: expected,
                })
            }
        }
    }

    pub(super) fn check_exhaustive(
        &mut self,
        scrutinee_type: &Type,
        arms: &[HirMatchArm],
        span: SourceSpan,
    ) {
        let applied = self.subs.apply(scrutinee_type);
        let has_catch_all = arms.iter().any(|arm| {
            arm.guard.is_none()
                && matches!(
                    arm.pattern.kind,
                    HirPatternKind::Wildcard(_) | HirPatternKind::Ident { .. }
                )
        });
        if has_catch_all {
            return;
        }
        if let Type::Named(name) = &applied {
            if let Some(HirItemKind::Enum(e)) = resolve_named_type(name, &self.types) {
                let mut covered = vec![false; e.variants.len()];
                for arm in arms {
                    if arm.guard.is_some() {
                        continue;
                    }
                    if let HirPatternKind::EnumVariant { variant_index, .. } = &arm.pattern.kind
                        && *variant_index < covered.len()
                    {
                        covered[*variant_index] = true;
                    }
                }
                if covered.iter().all(|c| *c) {
                    return;
                }
            }
        } else if let Type::Primitive(Primitive::Bool) = &applied {
            let mut has_true = false;
            let mut has_false = false;
            for arm in arms {
                if arm.guard.is_some() {
                    continue;
                }
                if let HirPatternKind::Literal {
                    value: LiteralValue::Bool(value),
                    ..
                } = &arm.pattern.kind
                {
                    if *value {
                        has_true = true;
                    } else {
                        has_false = true;
                    }
                }
            }
            if has_true && has_false {
                return;
            }
        }
        self.errors.push(
            self.source
                .error(span, TypeDiagnosticKind::NonExhaustiveMatch),
        );
    }
}
