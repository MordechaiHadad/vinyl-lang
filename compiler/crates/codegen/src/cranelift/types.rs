use std::collections::HashMap;

use cranelift_codegen::ir::{self, AbiParam, Signature, types};
use cranelift_codegen::isa::CallConv;

use vinyl_parser::ast::types::Primitive;
use vinyl_typecheck::hir::{HirAssignTarget, HirEnumVariantData, HirFunction, HirItemKind, Type};

pub fn extract_array_element_type(target: &HirAssignTarget) -> Option<&Type> {
    match target {
        HirAssignTarget::Index { array, .. } => {
            if let Type::Array { element, .. } = &array.type_ {
                Some(element.as_ref())
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn param_type_to_clif(t: &Type, pointer_type: ir::Type) -> ir::Type {
    match t {
        Type::Primitive(Primitive::Int32) => types::I32,
        Type::Primitive(Primitive::Int64 | Primitive::Int) => types::I64,
        Type::Primitive(Primitive::Int128) | Type::Primitive(Primitive::UInt128) => types::I128,
        Type::Primitive(Primitive::ISize) | Type::Ref(_) => pointer_type,
        Type::Primitive(Primitive::USize) => pointer_type,
        Type::Primitive(Primitive::UInt64 | Primitive::UInt) => types::I64,
        Type::Primitive(Primitive::Float32) => types::F32,
        Type::Primitive(Primitive::Float64 | Primitive::Float) => types::F64,
        Type::Primitive(Primitive::Bool) => types::I8,
        Type::Primitive(Primitive::Char) => types::I32,
        _ => types::I64,
    }
}

pub fn ir_type_from_primitive(t: &Type, pointer_type: ir::Type) -> ir::Type {
    match t {
        Type::Primitive(Primitive::Int32) => types::I32,
        Type::Primitive(Primitive::Int64 | Primitive::Int) => types::I64,
        Type::Primitive(Primitive::Int128) | Type::Primitive(Primitive::UInt128) => types::I128,
        Type::Primitive(Primitive::ISize) | Type::Ref(_) => pointer_type,
        Type::Primitive(Primitive::USize) => pointer_type,
        Type::Primitive(Primitive::UInt64 | Primitive::UInt) => types::I64,
        Type::Primitive(Primitive::Float32) => types::F32,
        Type::Primitive(Primitive::Float64 | Primitive::Float) => types::F64,
        Type::Primitive(Primitive::Bool) => types::I8,
        Type::Primitive(Primitive::Char) => types::I32,
        _ => types::I64,
    }
}

pub fn hir_sig_to_clif(
    func: &HirFunction,
    types: &HashMap<String, HirItemKind>,
    pointer_type: ir::Type,
) -> Signature {
    #[cfg(windows)]
    let call_conv = CallConv::WindowsFastcall;
    #[cfg(not(windows))]
    let call_conv = CallConv::SystemV;

    let mut sig = Signature::new(call_conv);
    let ptr_size = pointer_type.bytes();

    // Aggregates >16 bytes are returned through a hidden pointer (sret).
    let needs_sret = crate::layout::is_aggregate(&func.return_type)
        && crate::layout::aggregate_register_count(&func.return_type, types, ptr_size) == 0;
    if needs_sret {
        sig.params.push(AbiParam::new(pointer_type));
    }

    for param in &func.params {
        if crate::layout::is_aggregate(&param.type_) {
            let chunks = crate::layout::aggregate_register_count(&param.type_, types, ptr_size);
            if chunks == 0 {
                // >16 bytes: pass by reference
                sig.params.push(AbiParam::new(pointer_type));
            } else {
                for _ in 0..chunks {
                    sig.params.push(AbiParam::new(types::I64));
                }
            }
        } else {
            sig.params.push(AbiParam::new(param_type_to_clif(
                &param.type_,
                pointer_type,
            )));
        }
    }

    if !needs_sret {
        match &func.return_type {
            Type::Primitive(Primitive::Unit) => {}
            other if crate::layout::is_aggregate(other) => {
                let chunks = crate::layout::aggregate_register_count(other, types, ptr_size);
                for _ in 0..chunks {
                    sig.returns.push(AbiParam::new(types::I64));
                }
            }
            other => {
                sig.returns
                    .push(AbiParam::new(param_type_to_clif(other, pointer_type)));
            }
        }
    }

    sig
}

pub fn is_large_aggregate(
    t: &Type,
    types: &HashMap<String, HirItemKind>,
    pointer_size: u32,
) -> bool {
    // Internal representation: aggregates larger than 8 bytes live in memory
    // (pointer); smaller ones are packed into a single 64-bit value. Scalars
    // (including 128-bit ints) are never memory-backed.
    crate::layout::is_aggregate(t) && crate::layout::size_of(t, types, pointer_size) > 8
}

/// Whether equality for `t` must walk fields instead of comparing whole
/// bytes: aggregates containing floats (which need IEEE semantics, not bitwise)
/// and arrays (which are always memory-backed, so a packed i64 compare would
/// compare addresses). Heap types will extend this when they gain deep
/// equality in the std runtime.
pub fn type_needs_custom_equality(t: &Type, types: &HashMap<String, HirItemKind>) -> bool {
    match t {
        Type::Array { .. } => true,
        Type::Primitive(p) => matches!(
            p,
            Primitive::Float32 | Primitive::Float64 | Primitive::Float
        ),
        Type::Tuple(elements) => elements
            .iter()
            .any(|element| type_needs_custom_equality(element, types)),
        Type::Named(name) => named_type_needs_custom_equality(name, types, &mut Vec::new()),
        _ => false,
    }
}

fn named_type_needs_custom_equality(
    name: &str,
    types: &HashMap<String, HirItemKind>,
    visited: &mut Vec<String>,
) -> bool {
    if visited.iter().any(|n| n == name) {
        return false;
    }
    visited.push(name.to_string());
    let result = match types.get(name) {
        Some(HirItemKind::Struct(s)) => s
            .fields
            .iter()
            .any(|f| type_needs_custom_equality(&f.type_, types)),
        Some(HirItemKind::Enum(e)) => e.variants.iter().any(|variant| match &variant.data {
            Some(HirEnumVariantData::Tuple(element_types)) => element_types
                .iter()
                .any(|t| type_needs_custom_equality(t, types)),
            Some(HirEnumVariantData::Struct(fields)) => fields
                .iter()
                .any(|f| type_needs_custom_equality(&f.type_, types)),
            None => false,
        }),
        Some(HirItemKind::TupleStruct(t)) => {
            t.types.iter().any(|t| type_needs_custom_equality(t, types))
        }
        Some(HirItemKind::TypeAlias(alias)) => type_needs_custom_equality(&alias.type_, types),
        _ => false,
    };
    visited.pop();
    result
}
