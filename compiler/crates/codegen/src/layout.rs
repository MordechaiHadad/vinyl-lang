use std::collections::HashMap;

use vinyl_parser::ast::types::Primitive;
use vinyl_typecheck::hir::{HirEnumVariantData, HirItemKind, Type};

pub struct Layout {
    pub size: u32,
    pub alignment: u32,
}

pub struct FieldLayout {
    pub offset: u32,
    pub size: u32,
}

pub fn size_of(t: &Type, types: &HashMap<String, HirItemKind>, pointer_size: u32) -> u32 {
    match t {
        Type::Primitive(p) => match p {
            Primitive::Int8 | Primitive::UInt8 | Primitive::Bool => 1,
            Primitive::Int16 | Primitive::UInt16 => 2,
            Primitive::Int32 | Primitive::UInt32 | Primitive::Float32 | Primitive::Char => 4,
            Primitive::Int64 | Primitive::UInt64 | Primitive::Float64 => 8,
            Primitive::Int128 | Primitive::UInt128 => 16,
            Primitive::ISize | Primitive::USize | Primitive::String => pointer_size,
            Primitive::Unit => 0,
        },
        Type::Named(name) => named_type_size_of(name, types, pointer_size),
        Type::Ref(_) => pointer_size,
        Type::Array { element, size } => size_of(element, types, pointer_size) * (*size as u32),
        Type::Tuple(elements) => {
            if elements.is_empty() {
                return 0;
            }
            let mut total: u32 = 0;
            let max_align = elements
                .iter()
                .map(|e| align_of(e, types, pointer_size))
                .max()
                .unwrap_or(1);
            for element in elements {
                let elem_align = align_of(element, types, pointer_size);
                total = align_up(total, elem_align);
                total += size_of(element, types, pointer_size);
            }
            align_up(total, max_align)
        }
        Type::Generic { .. } | Type::Var(_) => pointer_size,
    }
}

pub fn align_of(t: &Type, types: &HashMap<String, HirItemKind>, pointer_size: u32) -> u32 {
    match t {
        Type::Primitive(p) => match p {
            Primitive::Int8 | Primitive::UInt8 | Primitive::Bool => 1,
            Primitive::Int16 | Primitive::UInt16 => 2,
            Primitive::Int32 | Primitive::UInt32 | Primitive::Float32 | Primitive::Char => 4,
            Primitive::Int64 | Primitive::UInt64 | Primitive::Float64 => 8,
            Primitive::Int128 | Primitive::UInt128 => 16,
            Primitive::ISize | Primitive::USize | Primitive::String => pointer_size,
            Primitive::Unit => 1,
        },
        Type::Named(name) => named_type_align_of(name, types, pointer_size),
        Type::Ref(_) => pointer_size,
        Type::Array { element, .. } => align_of(element, types, pointer_size),
        Type::Tuple(elements) => elements
            .iter()
            .map(|e| align_of(e, types, pointer_size))
            .max()
            .unwrap_or(1),
        Type::Generic { .. } | Type::Var(_) => pointer_size,
    }
}

fn named_type_size_of(name: &str, types: &HashMap<String, HirItemKind>, pointer_size: u32) -> u32 {
    match types.get(name) {
        Some(HirItemKind::Struct(s)) => {
            let field_types: Vec<(String, Type)> = s
                .fields
                .iter()
                .map(|f| (f.name.clone(), f.type_.clone()))
                .collect();
            let (total_size, _) = struct_layout(&field_types, s.repr_c, types, pointer_size);
            total_size
        }
        Some(HirItemKind::Enum(e)) => {
            let variant_data: Vec<Vec<Type>> = e
                .variants
                .iter()
                .map(|v| match &v.data {
                    Some(HirEnumVariantData::Tuple(types)) => types.clone(),
                    Some(HirEnumVariantData::Struct(fields)) => {
                        fields.iter().map(|f| f.type_.clone()).collect()
                    }
                    None => Vec::new(),
                })
                .collect();
            let (total_size, _, _) = enum_layout(&variant_data, types, pointer_size);
            total_size
        }
        Some(HirItemKind::TupleStruct(t)) => {
            size_of(&Type::Tuple(t.types.clone()), types, pointer_size)
        }
        // todo: cycle detection for recursive named types, stack overflow guard
        _ => pointer_size,
    }
}

fn named_type_align_of(name: &str, types: &HashMap<String, HirItemKind>, pointer_size: u32) -> u32 {
    match types.get(name) {
        Some(HirItemKind::Struct(s)) => s
            .fields
            .iter()
            .map(|f| align_of(&f.type_, types, pointer_size))
            .max()
            .unwrap_or(1),
        Some(HirItemKind::Enum(e)) => {
            let mut max_align = 1u32;
            for variant in &e.variants {
                let variant_types = match &variant.data {
                    Some(HirEnumVariantData::Tuple(tys)) => tys.clone(),
                    Some(HirEnumVariantData::Struct(fields)) => {
                        fields.iter().map(|f| f.type_.clone()).collect()
                    }
                    None => continue,
                };
                for t in &variant_types {
                    max_align = max_align.max(align_of(t, types, pointer_size));
                }
            }
            max_align
        }
        Some(HirItemKind::TupleStruct(t)) => {
            align_of(&Type::Tuple(t.types.clone()), types, pointer_size)
        }
        _ => pointer_size,
    }
}

pub fn tuple_field_offset(
    index: usize,
    elements: &[Type],
    types: &HashMap<String, HirItemKind>,
    pointer_size: u32,
) -> u32 {
    if elements.is_empty() {
        return 0;
    }
    let mut offset: u32 = 0;
    for (i, element) in elements.iter().enumerate() {
        let elem_align = align_of(element, types, pointer_size);
        offset = align_up(offset, elem_align);
        if i == index {
            return offset;
        }
        offset += size_of(element, types, pointer_size);
    }
    0
}

pub fn struct_layout(
    fields: &[(String, Type)],
    repr_c: bool,
    types: &HashMap<String, HirItemKind>,
    pointer_size: u32,
) -> (u32, Vec<(String, FieldLayout)>) {
    if fields.is_empty() {
        return (0, Vec::new());
    }

    let mut indexed: Vec<(usize, &str, &Type)> = fields
        .iter()
        .enumerate()
        .map(|(i, (name, t))| (i, name.as_str(), t))
        .collect();

    if !repr_c {
        indexed.sort_by(|(_, _, a), (_, _, b)| {
            align_of(b, types, pointer_size).cmp(&align_of(a, types, pointer_size))
        });
    }

    let mut current_offset: u32 = 0;
    let mut max_align: u32 = 1;
    let mut result: Vec<(String, FieldLayout)> = Vec::with_capacity(fields.len());

    for (_, name, t) in &indexed {
        let field_align = align_of(t, types, pointer_size);
        max_align = max_align.max(field_align);
        current_offset = align_up(current_offset, field_align);
        let field_size = size_of(t, types, pointer_size);
        result.push((
            (*name).to_string(),
            FieldLayout {
                offset: current_offset,
                size: field_size,
            },
        ));
        current_offset += field_size;
    }

    let total_size = align_up(current_offset, max_align);
    (total_size, result)
}

pub fn enum_layout(
    variant_data: &[Vec<Type>],
    types: &HashMap<String, HirItemKind>,
    pointer_size: u32,
) -> (u32, u32, u32) {
    let num_variants = variant_data.len();
    if num_variants == 0 {
        return (1, 0, 1);
    }

    // todo: niche/discriminant overlap optimization for Option-like enums
    let discriminant_size = if num_variants <= 256 { 1u32 } else { 2u32 };
    let mut max_data_size = 0u32;
    let mut max_data_align = 1u32;

    for variant_types in variant_data {
        let mut variant_size = 0u32;
        let mut variant_align = 1u32;
        for t in variant_types {
            let elem_align = align_of(t, types, pointer_size);
            variant_align = variant_align.max(elem_align);
            variant_size = align_up(variant_size, elem_align);
            variant_size += size_of(t, types, pointer_size);
        }
        variant_size = align_up(variant_size, variant_align);
        max_data_size = max_data_size.max(variant_size);
        max_data_align = max_data_align.max(variant_align);
    }

    let data_offset = align_up(discriminant_size, max_data_align);
    let total_size = align_up(data_offset + max_data_size, max_data_align);

    (total_size, data_offset, discriminant_size)
}

pub fn align_up(offset: u32, alignment: u32) -> u32 {
    if alignment <= 1 {
        return offset;
    }
    let mask = alignment - 1;
    (offset + mask) & !mask
}
