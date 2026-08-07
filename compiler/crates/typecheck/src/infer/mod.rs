use std::collections::{BTreeMap, HashMap};

use miette::{NamedSource, SourceSpan};
use vinyl_parser::ast::expression::Expression;
use vinyl_parser::ast::item::{EnumVariantData, FunctionDef, Item};
use vinyl_parser::ast::statement::Statement;
use vinyl_parser::ast::types::{Primitive, Type as AstType};

use crate::error::{InferResult, TypeDiagnostic, TypeDiagnosticKind};
use crate::hir::{
    HirEnum, HirEnumVariant, HirEnumVariantData, HirField, HirItem, HirItemKind, HirStruct,
    HirTupleStruct, HirTypeAlias, Type,
};
use crate::module::{ModuleTable, resolve_module};

use crate::index::builder::IndexBuilder;
pub use crate::index::{Definition, DefinitionKind, HirExprRef, TypeckResult};

pub mod expression;
pub mod literal;
pub mod pattern;
pub mod resolve;
pub mod scope;
pub mod statement;
pub mod unify;

use scope::ScopeState;
use unify::SubstitutionState;

#[derive(Debug, Clone)]
struct TypeScheme {
    type_: Type,
    mutable: bool,
}

pub(super) struct SourceContext {
    source: String,
    source_name: String,
}

impl SourceContext {
    fn new(source: &str, source_name: &str) -> Self {
        SourceContext {
            source: source.to_string(),
            source_name: source_name.to_string(),
        }
    }

    pub(super) fn error(&self, span: SourceSpan, kind: TypeDiagnosticKind) -> TypeDiagnostic {
        TypeDiagnostic {
            kind,
            source_code: NamedSource::new(&self.source_name, self.source.to_string()),
            span,
        }
    }

    pub(super) fn type_mismatch(
        &self,
        span: SourceSpan,
        expected: Type,
        found: Type,
    ) -> TypeDiagnostic {
        TypeDiagnostic {
            kind: TypeDiagnosticKind::Mismatch { expected, found },
            source_code: NamedSource::new(&self.source_name, self.source.to_string()),
            span,
        }
    }
}

struct InferState {
    source: SourceContext,
    scope: ScopeState,
    types: HashMap<String, HirItemKind>,
    subs: SubstitutionState,
    current_return_type: Option<Type>,
    loop_depth: usize,
    errors: Vec<TypeDiagnostic>,
    module_table: ModuleTable,
    type_origins: HashMap<String, String>,
}

impl InferState {
    fn new(source: &str, source_name: &str, module_table: &ModuleTable) -> Self {
        let mut type_origins = HashMap::new();
        for (module, exports) in module_table {
            if exports.imported {
                for type_name in &exports.types {
                    type_origins.insert(type_name.clone(), module.clone());
                }
            }
        }
        InferState {
            source: SourceContext::new(source, source_name),
            scope: ScopeState::new(),
            types: HashMap::new(),
            subs: SubstitutionState::new(),
            current_return_type: None,
            loop_depth: 0,
            errors: Vec::new(),
            module_table: module_table.clone(),
            type_origins,
        }
    }

    pub(super) fn canonicalize_scoped_name(
        &mut self,
        name: &str,
        span: SourceSpan,
    ) -> Result<String, Box<TypeDiagnostic>> {
        let segments: Vec<String> = name.split("::").map(str::to_string).collect();
        let Some((module_len, exports)) = resolve_module(&self.module_table, &segments) else {
            if segments.len() > 1 {
                let module = segments[..segments.len() - 1].join("::");
                return Err(Box::new(
                    self.source
                        .error(span, TypeDiagnosticKind::UndefinedModule { name: module }),
                ));
            }
            return Ok(name.to_string());
        };
        let type_name = segments[module_len..].join("::");
        if exports.imported && exports.types.iter().any(|t| t == &type_name) {
            Ok(type_name)
        } else if !exports.imported {
            Err(Box::new(self.source.error(
                span,
                TypeDiagnosticKind::MissingImport {
                    module: segments[..module_len].join("::"),
                    name: type_name,
                    import_path: exports.import_path.clone(),
                },
            )))
        } else {
            Err(Box::new(self.source.error(
                span,
                TypeDiagnosticKind::PrivateAccess {
                    module: segments[..module_len].join("::"),
                    name: type_name,
                },
            )))
        }
    }

    fn canonicalize_type(
        &mut self,
        type_: &AstType,
        span: SourceSpan,
    ) -> Result<AstType, Box<TypeDiagnostic>> {
        match type_ {
            AstType::Named(name) => Ok(AstType::Named(self.canonicalize_scoped_name(name, span)?)),
            AstType::Generic { name, args } => {
                let args = args
                    .iter()
                    .map(|arg| self.canonicalize_type(arg, span))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(AstType::Generic {
                    name: name.clone(),
                    args,
                })
            }
            AstType::Array { element, size } => Ok(AstType::Array {
                element: Box::new(self.canonicalize_type(element, span)?),
                size: *size,
            }),
            AstType::Ref(inner) => Ok(AstType::Ref(Box::new(self.canonicalize_type(inner, span)?))),
            AstType::Tuple(elements) => Ok(AstType::Tuple(
                elements
                    .iter()
                    .map(|element| self.canonicalize_type(element, span))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            AstType::Primitive(_) | AstType::Var(_) => Ok(type_.clone()),
        }
    }

    fn canonicalize_item_types(&mut self, item: &mut Item) {
        let span = item.span();
        match item {
            Item::Struct(s) => {
                for field in &mut s.fields {
                    match self.canonicalize_type(&field.type_, span) {
                        Ok(type_) => field.type_ = type_,
                        Err(e) => self.errors.push(*e),
                    }
                }
            }
            Item::TupleStruct(t) => {
                for type_ in &mut t.types {
                    match self.canonicalize_type(type_, span) {
                        Ok(canonical) => *type_ = canonical,
                        Err(e) => self.errors.push(*e),
                    }
                }
            }
            Item::Enum(e) => {
                for variant in &mut e.variants {
                    if let Some(data) = &mut variant.data {
                        match data {
                            EnumVariantData::Tuple(types) => {
                                for type_ in types {
                                    match self.canonicalize_type(type_, span) {
                                        Ok(canonical) => *type_ = canonical,
                                        Err(e) => self.errors.push(*e),
                                    }
                                }
                            }
                            EnumVariantData::Struct(fields) => {
                                for field in fields {
                                    match self.canonicalize_type(&field.type_, span) {
                                        Ok(type_) => field.type_ = type_,
                                        Err(e) => self.errors.push(*e),
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Item::TypeAlias(a) => match self.canonicalize_type(&a.type_, span) {
                Ok(type_) => a.type_ = type_,
                Err(e) => self.errors.push(*e),
            },
            Item::Function(f) => {
                for param in &mut f.params {
                    match self.canonicalize_type(&param.type_, span) {
                        Ok(type_) => param.type_ = type_,
                        Err(e) => self.errors.push(*e),
                    }
                }
                if let Some(return_type) = &mut f.return_type {
                    match self.canonicalize_type(return_type, span) {
                        Ok(type_) => *return_type = type_,
                        Err(e) => self.errors.push(*e),
                    }
                }
            }
            Item::Import(_) => {}
        }
    }
}

pub(super) fn resolve_named_type<'a>(
    name: &str,
    types: &'a HashMap<String, HirItemKind>,
) -> Option<&'a HirItemKind> {
    match types.get(name) {
        Some(HirItemKind::TypeAlias(a)) => match &a.type_ {
            Type::Named(target) => resolve_named_type(target, types),
            _ => None,
        },
        other => other,
    }
}

enum AliasTargetError {
    UnknownType(String),
    Recursive,
}

fn validate_type_aliases(state: &mut InferState) {
    let aliases: Vec<(SourceSpan, String, Type)> = state
        .types
        .iter()
        .filter_map(|(name, kind)| {
            if let HirItemKind::TypeAlias(a) = kind {
                Some((a.span, name.clone(), a.type_.clone()))
            } else {
                None
            }
        })
        .collect();
    for (span, name, target) in aliases {
        match validate_alias_target(&target, &state.types, &mut Vec::new()) {
            Err(AliasTargetError::UnknownType(t)) => state.errors.push(
                state
                    .source
                    .error(span, TypeDiagnosticKind::UndefinedType { name: t }),
            ),
            Err(AliasTargetError::Recursive) => state.errors.push(
                state
                    .source
                    .error(span, TypeDiagnosticKind::RecursiveTypeAlias { name }),
            ),
            Ok(()) => {}
        }
    }
}

fn validate_alias_target(
    target: &Type,
    types: &HashMap<String, HirItemKind>,
    visited: &mut Vec<String>,
) -> Result<(), AliasTargetError> {
    match target {
        Type::Named(n) => {
            if visited.contains(n) {
                return Err(AliasTargetError::Recursive);
            }
            match types.get(n) {
                Some(HirItemKind::TypeAlias(a)) => {
                    visited.push(n.clone());
                    let result = validate_alias_target(&a.type_, types, visited);
                    visited.pop();
                    result
                }
                Some(_) => Ok(()),
                None => Err(AliasTargetError::UnknownType(n.clone())),
            }
        }
        _ => Ok(()),
    }
}

fn validate_named_types(state: &mut InferState, items: &[Item]) {
    fn check_type(state: &mut InferState, type_: &AstType, span: SourceSpan) {
        match type_ {
            AstType::Named(name) if !name.contains("::") && !state.types.contains_key(name) => {
                state.errors.push(state.source.error(
                    span,
                    TypeDiagnosticKind::UndefinedType { name: name.clone() },
                ));
            }
            AstType::Ref(inner) => check_type(state, inner, span),
            AstType::Array { element, .. } => check_type(state, element, span),
            AstType::Tuple(elements) => {
                for element in elements {
                    check_type(state, element, span);
                }
            }
            AstType::Generic { args, .. } => {
                for arg in args {
                    check_type(state, arg, span);
                }
            }
            _ => {}
        }
    }

    fn check_expr(state: &mut InferState, expr: &Expression) {
        match expr {
            Expression::Block(block, _) => check_stmts(state, block),
            Expression::If {
                then_block,
                else_if,
                else_block,
                ..
            } => {
                check_stmts(state, then_block);
                for (_, block) in else_if {
                    check_stmts(state, block);
                }
                if let Some(block) = else_block {
                    check_stmts(state, block);
                }
            }
            _ => {}
        }
    }

    fn check_stmts(state: &mut InferState, stmts: &[Statement]) {
        for stmt in stmts {
            match stmt {
                Statement::Let {
                    type_: Some(type_),
                    span,
                    ..
                } => check_type(state, type_, *span),
                Statement::Expression(expr) => check_expr(state, expr),
                Statement::Return(Some(expr), _) => check_expr(state, expr),
                Statement::Value(expr, _) => check_expr(state, expr),
                Statement::If {
                    then_block,
                    else_if,
                    else_block,
                    ..
                } => {
                    check_stmts(state, then_block);
                    for (_, block) in else_if {
                        check_stmts(state, block);
                    }
                    if let Some(block) = else_block {
                        check_stmts(state, block);
                    }
                }
                Statement::While { body, .. } | Statement::Loop { body, .. } => {
                    check_stmts(state, body);
                }
                _ => {}
            }
        }
    }

    for item in items {
        let span = item.span();
        match item {
            Item::Struct(s) => {
                for field in &s.fields {
                    check_type(state, &field.type_, span);
                }
            }
            Item::TupleStruct(t) => {
                for type_ in &t.types {
                    check_type(state, type_, span);
                }
            }
            Item::Enum(e) => {
                for variant in &e.variants {
                    if let Some(data) = &variant.data {
                        match data {
                            EnumVariantData::Tuple(types) => {
                                for type_ in types {
                                    check_type(state, type_, span);
                                }
                            }
                            EnumVariantData::Struct(fields) => {
                                for field in fields {
                                    check_type(state, &field.type_, span);
                                }
                            }
                        }
                    }
                }
            }
            Item::TypeAlias(a) => check_type(state, &a.type_, span),
            Item::Function(f) => {
                for param in &f.params {
                    check_type(state, &param.type_, span);
                }
                if let Some(return_type) = &f.return_type {
                    check_type(state, return_type, span);
                }
                check_stmts(state, &f.body);
            }
            Item::Import(_) => {}
        }
    }
}

fn field_types_of<'a>(types: &'a HashMap<String, HirItemKind>, name: &str) -> Vec<&'a Type> {
    match types.get(name) {
        Some(HirItemKind::Struct(s)) => s.fields.iter().map(|f| &f.type_).collect(),
        Some(HirItemKind::TupleStruct(t)) => t.types.iter().collect(),
        Some(HirItemKind::Enum(e)) => e
            .variants
            .iter()
            .flat_map(|v| match &v.data {
                Some(HirEnumVariantData::Tuple(tys)) => tys.iter().collect::<Vec<_>>(),
                Some(HirEnumVariantData::Struct(fields)) => {
                    fields.iter().map(|f| &f.type_).collect()
                }
                None => Vec::new(),
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Collect named types reached by value (through fields, array elements, and
/// tuples) but not through references.
fn by_value_named(t: &Type) -> Vec<&Type> {
    match t {
        Type::Named(_) => vec![t],
        Type::Array { element, .. } => by_value_named(element),
        Type::Tuple(elements) => elements.iter().flat_map(by_value_named).collect(),
        _ => Vec::new(),
    }
}

/// Resolve a type alias chain to the underlying named type (or the alias name
/// itself if the chain bottoms out elsewhere). Cycle-safe for defensive use.
fn resolve_named_alias<'a>(
    name: &'a str,
    types: &'a HashMap<String, HirItemKind>,
    visited: &mut Vec<String>,
) -> &'a str {
    if visited.iter().any(|v| v == name) {
        return name;
    }
    visited.push(name.to_string());
    let result = match types.get(name) {
        Some(HirItemKind::TypeAlias(a)) => match &a.type_ {
            Type::Named(target) => resolve_named_alias(target, types, visited),
            _ => name,
        },
        _ => name,
    };
    visited.pop();
    result
}

fn contains_by_value_cycle(
    start: &str,
    current: &str,
    types: &HashMap<String, HirItemKind>,
    in_path: &mut Vec<String>,
    alias_visited: &mut Vec<String>,
) -> bool {
    for field_type in field_types_of(types, current) {
        for contained in by_value_named(field_type) {
            let Type::Named(next) = contained else {
                continue;
            };
            let resolved = resolve_named_alias(next, types, alias_visited);
            if resolved == start {
                return true;
            }
            if in_path.iter().any(|p| p.as_str() == resolved) {
                continue;
            }
            in_path.push(resolved.to_string());
            if contains_by_value_cycle(start, resolved, types, in_path, alias_visited) {
                return true;
            }
            in_path.pop();
        }
    }
    false
}

/// Reject named types that contain themselves by value, which would have
/// infinite size (e.g. `struct A { b: B }` with `struct B { a: A }`, or the
/// same reached through a type alias like `type X = S; struct S { f: X }`).
fn validate_recursive_types(state: &mut InferState) {
    let named: Vec<(SourceSpan, String)> = state
        .types
        .iter()
        .filter_map(|(name, kind)| match kind {
            HirItemKind::Struct(s) => Some((s.span, name.clone())),
            HirItemKind::TupleStruct(t) => Some((t.span, name.clone())),
            HirItemKind::Enum(e) => Some((e.span, name.clone())),
            _ => None,
        })
        .collect();
    for (span, name) in named {
        let mut in_path = Vec::new();
        let mut alias_visited = Vec::new();
        if contains_by_value_cycle(&name, &name, &state.types, &mut in_path, &mut alias_visited) {
            state.errors.push(state.source.error(
                span,
                TypeDiagnosticKind::RecursiveType {
                    a: Type::Named(name.clone()),
                    b: Type::Named(name),
                },
            ));
        }
    }
}

pub fn typeck(
    items: &[Item],
    source: &str,
    source_name: &str,
) -> Result<(Vec<HirItem>, Vec<TypeDiagnostic>), Vec<TypeDiagnostic>> {
    typeck_with_modules(items, source, source_name, &ModuleTable::new())
}

pub fn validate_main_return_type(
    items: &[Item],
    source: &str,
    source_name: &str,
) -> InferResult<()> {
    let Some(function) = items.iter().find_map(|item| match item {
        Item::Function(function) if function.name == "main" => Some(function),
        _ => None,
    }) else {
        return Ok(());
    };
    if function
        .return_type
        .as_ref()
        .is_some_and(|return_type| !matches!(return_type, AstType::Primitive(Primitive::Unit)))
    {
        return Err(Box::new(
            SourceContext::new(source, source_name)
                .error(function.span, TypeDiagnosticKind::MainReturnType),
        ));
    }
    Ok(())
}

pub fn typeck_with_modules(
    items: &[Item],
    source: &str,
    source_name: &str,
    module_table: &ModuleTable,
) -> Result<(Vec<HirItem>, Vec<TypeDiagnostic>), Vec<TypeDiagnostic>> {
    let mut state = InferState::new(source, source_name, module_table);

    let mut owned_items = items.to_vec();
    for item in &mut owned_items {
        state.canonicalize_item_types(item);
    }

    let signatures: HashMap<&str, &FunctionDef> = owned_items
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

    for item in &owned_items {
        let hir_item = match item {
            Item::Struct(s) => Some(HirItem {
                span: s.span,
                kind: HirItemKind::Struct(HirStruct {
                    span: s.span,
                    name: s.name.clone(),
                    public: s.public,
                    repr_c: s.attrs.iter().any(|a| a.name == "repr_c"),
                    fields: s
                        .fields
                        .iter()
                        .map(|f| HirField {
                            span: f.span,
                            public: f.public,
                            name: f.name.clone(),
                            type_: f.type_.clone(),
                        })
                        .collect(),
                }),
            }),
            Item::TupleStruct(t) => Some(HirItem {
                span: t.span,
                kind: HirItemKind::TupleStruct(HirTupleStruct {
                    span: t.span,
                    name: t.name.clone(),
                    public: t.public,
                    types: t.types.clone(),
                }),
            }),
            Item::Enum(e) => Some(HirItem {
                span: e.span,
                kind: HirItemKind::Enum(HirEnum {
                    span: e.span,
                    name: e.name.clone(),
                    public: e.public,
                    variants: e
                        .variants
                        .iter()
                        .map(|v| HirEnumVariant {
                            span: v.span,
                            name: v.name.clone(),
                            data: v.data.as_ref().map(|d| match d {
                                EnumVariantData::Tuple(types) => {
                                    HirEnumVariantData::Tuple(types.clone())
                                }
                                EnumVariantData::Struct(fields) => HirEnumVariantData::Struct(
                                    fields
                                        .iter()
                                        .map(|f| HirField {
                                            span: f.span,
                                            public: f.public,
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
            Item::TypeAlias(a) => Some(HirItem {
                span: a.span,
                kind: HirItemKind::TypeAlias(HirTypeAlias {
                    span: a.span,
                    name: a.name.clone(),
                    public: a.public,
                    type_: a.type_.clone(),
                }),
            }),
            Item::Function(_) => None,
            Item::Import(_) => None,
        };
        if let Some(hir) = hir_item {
            let name = match &hir.kind {
                HirItemKind::Struct(s) => s.name.clone(),
                HirItemKind::TupleStruct(t) => t.name.clone(),
                HirItemKind::Enum(e) => e.name.clone(),
                HirItemKind::TypeAlias(a) => a.name.clone(),
                _ => unreachable!(),
            };
            state.types.insert(name, hir.kind.clone());
            hir_items.push(hir);
        }
    }

    for (module, exports) in state.module_table.clone() {
        if !exports.imported {
            continue;
        }
        for type_name in &exports.types {
            let scoped_name = format!("{module}::{type_name}");
            if let Some(kind) = state.types.get(&scoped_name).cloned() {
                state.types.entry(type_name.clone()).or_insert(kind);
            }
        }
    }

    validate_named_types(&mut state, &owned_items);
    validate_type_aliases(&mut state);
    validate_recursive_types(&mut state);

    for item in &owned_items {
        if let Item::Function(f) = item {
            match state.infer_function(f, &signatures) {
                Ok(hir) => hir_items.push(HirItem {
                    span: f.span,
                    kind: HirItemKind::Function(hir),
                }),
                Err(e) => state.errors.push(*e),
            }
        }
    }

    let all_diagnostics = std::mem::take(&mut state.errors);
    let mut fatal_errors = Vec::new();
    let mut warnings = Vec::new();
    for diagnostic in all_diagnostics {
        if matches!(diagnostic.kind, TypeDiagnosticKind::UnreachableStatement) {
            warnings.push(diagnostic);
        } else {
            fatal_errors.push(diagnostic);
        }
    }
    if fatal_errors.is_empty() {
        Ok((hir_items, warnings))
    } else {
        Err(fatal_errors)
    }
}

pub fn typeck_with_index(
    items: &[Item],
    source: &str,
    source_name: &str,
    module_table: &ModuleTable,
) -> Result<(TypeckResult, Vec<TypeDiagnostic>), Vec<TypeDiagnostic>> {
    let (hir_items, warnings) = typeck_with_modules(items, source, source_name, module_table)?;
    let index = IndexBuilder::default().build(&hir_items);
    Ok((
        TypeckResult {
            items: hir_items,
            expr_at_pos: index.expr_at_pos,
            definitions: index.definitions,
            references: index.references,
            unused: index.unused,
            type_positions: collect_type_positions(items, source),
            field_accesses: index.field_accesses,
        },
        warnings,
    ))
}

fn collect_type_positions(items: &[Item], source: &str) -> BTreeMap<usize, String> {
    let mut positions = BTreeMap::new();
    for item in items {
        match item {
            Item::Function(f) => {
                if let Some(offset) = return_type_offset(source, f.span) {
                    positions.insert(
                        offset,
                        f.return_type
                            .as_ref()
                            .map(ToString::to_string)
                            .unwrap_or_default(),
                    );
                }
                for param in &f.params {
                    if let Some(offset) = type_after_colon(source, param.span) {
                        positions.insert(offset, param.type_.to_string());
                    }
                }
                for stmt in &f.body {
                    collect_statement_type_positions(stmt, source, &mut positions);
                }
            }
            Item::Struct(s) => {
                for field in &s.fields {
                    if let Some(offset) = type_after_colon(source, field.span) {
                        positions.insert(offset, field.type_.to_string());
                    }
                }
            }
            Item::TupleStruct(t) => {
                collect_parenthesized_types(source, t.span, &t.types, &mut positions);
            }
            Item::Enum(e) => {
                for variant in &e.variants {
                    if let Some(data) = &variant.data {
                        match data {
                            EnumVariantData::Tuple(types) => {
                                collect_parenthesized_types(
                                    source,
                                    variant.span,
                                    types,
                                    &mut positions,
                                );
                            }
                            EnumVariantData::Struct(fields) => {
                                for field in fields {
                                    if let Some(offset) = type_after_colon(source, field.span) {
                                        positions.insert(offset, field.type_.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Item::TypeAlias(a) => {
                if let Some(offset) = type_after_colon(source, a.span) {
                    positions.insert(offset, a.type_.to_string());
                }
            }
            Item::Import(_) => {}
        }
    }
    positions
}

fn collect_statement_type_positions(
    stmt: &Statement,
    source: &str,
    positions: &mut BTreeMap<usize, String>,
) {
    match stmt {
        Statement::Let {
            span,
            type_: Some(type_),
            ..
        } => {
            if let Some(offset) = let_type_offset(source, *span) {
                positions.insert(offset, type_.to_string());
            }
        }
        Statement::Expression(Expression::Block(stmts, _)) => {
            for s in stmts {
                collect_statement_type_positions(s, source, positions);
            }
        }
        Statement::If {
            then_block,
            else_if,
            else_block,
            ..
        } => {
            for s in then_block {
                collect_statement_type_positions(s, source, positions);
            }
            for (_, block) in else_if {
                for s in block {
                    collect_statement_type_positions(s, source, positions);
                }
            }
            if let Some(block) = else_block {
                for s in block {
                    collect_statement_type_positions(s, source, positions);
                }
            }
        }
        Statement::While { body, .. } | Statement::Loop { body, .. } => {
            for s in body {
                collect_statement_type_positions(s, source, positions);
            }
        }
        _ => {}
    }
}

fn type_after_colon(source: &str, span: SourceSpan) -> Option<usize> {
    let text = source.get(span.offset()..span.offset() + span.len())?;
    let colon = text.find(':')?;
    let after = &text[colon + 1..];
    let start = after.find(|c: char| !c.is_whitespace())?;
    Some(span.offset() + colon + 1 + start)
}

fn let_type_offset(source: &str, span: SourceSpan) -> Option<usize> {
    let text = source.get(span.offset()..span.offset() + span.len())?;
    let before_eq = text.split('=').next()?;
    let colon = before_eq.find(':')?;
    let after = &before_eq[colon + 1..];
    let start = after.find(|c: char| !c.is_whitespace())?;
    Some(span.offset() + colon + 1 + start)
}

fn return_type_offset(source: &str, span: SourceSpan) -> Option<usize> {
    let text = source.get(span.offset()..span.offset() + span.len())?;
    let paren = text.find(')')?;
    let after_paren = &text[paren + 1..];
    let colon = after_paren.find(':')?;
    let after = &after_paren[colon + 1..];
    let start = after.find(|c: char| !c.is_whitespace())?;
    Some(span.offset() + paren + 1 + colon + 1 + start)
}

fn collect_parenthesized_types(
    source: &str,
    span: SourceSpan,
    types: &[Type],
    positions: &mut BTreeMap<usize, String>,
) {
    let Some(text) = source.get(span.offset()..span.offset() + span.len()) else {
        return;
    };
    let Some(paren) = text.find('(') else {
        return;
    };
    let mut cursor = paren + 1;
    for type_ in types {
        let rest = &text[cursor..];
        let Some(start) = rest.find(|c: char| !c.is_whitespace()) else {
            return;
        };
        cursor += start;
        let name = type_.to_string();
        positions.insert(span.offset() + cursor, name.clone());
        let rest = &text[cursor..];
        let Some(extent) = type_token_extent(rest) else {
            return;
        };
        cursor += extent;
        let rest = &text[cursor..];
        let Some(comma) = rest.find(',') else {
            return;
        };
        cursor += comma + 1;
    }
}

fn type_token_extent(text: &str) -> Option<usize> {
    let mut depth = 0u32;
    for (index, c) in text.char_indices() {
        match c {
            '(' | '[' | '<' => depth += 1,
            ')' | ']' | '>' => {
                if depth == 0 {
                    return Some(index);
                }
                depth -= 1;
            }
            ',' if depth == 0 => return Some(index),
            _ => {}
        }
    }
    Some(text.len())
}
