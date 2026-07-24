use std::collections::HashMap;

use cranelift_codegen::ir::{self, AbiParam, Signature, types};
use cranelift_codegen::isa::CallConv;

use vinyl_parser::ast::types::Primitive;
use vinyl_typecheck::hir::{HirAssignTarget, HirFunction, HirItemKind, Type};

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

pub fn element_byte_size(t: &Type, pointer_type: ir::Type) -> u32 {
    let ptr_size = pointer_type.bytes();
    match t {
        Type::Primitive(p) => match p {
            Primitive::Int8 | Primitive::UInt8 | Primitive::Bool => 1,
            Primitive::Int16 | Primitive::UInt16 => 2,
            Primitive::Int32 | Primitive::UInt32 | Primitive::Float32 | Primitive::Char => 4,
            Primitive::Int64 | Primitive::UInt64 | Primitive::Float64 => 8,
            Primitive::Int128 | Primitive::UInt128 => 16,
            Primitive::ISize | Primitive::USize | Primitive::String => ptr_size,
            Primitive::Unit => 0,
        },
        Type::Ref(_) => ptr_size,
        Type::Array { element, size } => element_byte_size(element, pointer_type) * (*size as u32),
        _ => ptr_size,
    }
}

pub fn param_type_to_clif(t: &Type, pointer_type: ir::Type) -> ir::Type {
    match t {
        Type::Primitive(Primitive::Int32) => types::I32,
        Type::Primitive(Primitive::Int64) => types::I64,
        Type::Primitive(Primitive::ISize) | Type::Ref(_) => pointer_type,
        Type::Primitive(Primitive::USize) => pointer_type,
        Type::Primitive(Primitive::Float64) => types::F64,
        Type::Primitive(Primitive::Bool) => types::I8,
        Type::Primitive(Primitive::Char) => types::I32,
        _ => types::I64,
    }
}

pub fn ir_type_from_primitive(t: &Type, pointer_type: ir::Type) -> ir::Type {
    match t {
        Type::Primitive(Primitive::Int32) => types::I32,
        Type::Primitive(Primitive::Int64) => types::I64,
        Type::Primitive(Primitive::ISize) | Type::Ref(_) => pointer_type,
        Type::Primitive(Primitive::USize) => pointer_type,
        Type::Primitive(Primitive::Float64) => types::F64,
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

    // SRet: hidden pointer for large aggregate return
    // todo: baseline sret, multi-register return replaces this
    let needs_sret = match &func.return_type {
        Type::Primitive(Primitive::Unit) => false,
        other => crate::layout::size_of(other, types, ptr_size) > 8,
    };
    if needs_sret {
        sig.params.push(AbiParam::new(pointer_type));
    }

    for param in &func.params {
        let param_size = crate::layout::size_of(&param.type_, types, ptr_size);
        if param_size > 8 {
            // todo: baseline by-ref, multi-register decomposition replaces this
            sig.params.push(AbiParam::new(pointer_type));
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
            other => {
                sig.returns.push(AbiParam::new(param_type_to_clif(
                    other,
                    pointer_type,
                )));
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
    // todo: single-register baseline, decompose into multi-reg for perf
    crate::layout::size_of(t, types, pointer_size) > 8
}
