use vinyl_codegen::CodegenBackend;
use vinyl_codegen::cranelift::CraneliftBackend;

pub fn run(source: &str) -> Result<i64, String> {
    let source = wrap_main_with_print(source)?;
    let items =
        vinyl_parser::parse_and_lower(&source).map_err(|e| format!("parser error: {e:?}"))?;
    let (hir, _warnings) =
        vinyl_typecheck::typeck(&items, &source, "<test>").map_err(|_| "type error")?;
    let mut backend = CraneliftBackend::new().map_err(|e| format!("backend error: {e}"))?;
    backend
        .compile(&hir)
        .map_err(|e| format!("compile error: {e}"))?;
    vinyl_codegen::runtime::begin_capture();
    let result = backend.run().map_err(|e| format!("run error: {e}"))?;
    let output = vinyl_codegen::runtime::take_output();
    if output.trim().is_empty() {
        return Ok(result);
    }
    let output = output.trim();
    if source.contains("fn __test_main(): char") {
        return output
            .chars()
            .next()
            .map(|character| character as i64)
            .ok_or_else(|| "output error: empty char".to_string());
    }
    if source.contains("fn __test_main(): bool") {
        return match output {
            "true" => Ok(1),
            "false" => Ok(0),
            _ => Err(format!("output error: {output}")),
        };
    }
    output
        .parse()
        .or_else(|_| {
            output
                .chars()
                .next()
                .ok_or(())
                .map(|character| character as i64)
        })
        .map_err(|_| format!("output error: {output}"))
}

fn wrap_main_with_print(source: &str) -> Result<String, String> {
    let Some(main_start) = source.find("fn main") else {
        return Ok(source.to_string());
    };
    let Some(body_start) = source[main_start..]
        .find('{')
        .map(|offset| main_start + offset)
    else {
        return Err("main body not found".to_string());
    };
    matching_brace(source, body_start)?;
    let declaration = &source[main_start..body_start];
    if declaration.trim_end().ends_with("fn main()") {
        return Ok(source.to_string());
    }
    let return_type = declaration
        .split_once(':')
        .map(|(_, return_type)| return_type.trim());
    if !return_type.is_some_and(is_printable_type) {
        return Ok(source.to_string());
    }
    let mut wrapped = source.to_string();
    wrapped.replace_range(main_start..main_start + "fn main".len(), "fn __test_main");
    wrapped.push_str("\nfn main() { println(__test_main()); }\n");
    Ok(wrapped)
}

fn is_printable_type(return_type: &str) -> bool {
    matches!(
        return_type,
        "int"
            | "int8"
            | "int16"
            | "int32"
            | "int64"
            | "int128"
            | "isize"
            | "uint8"
            | "uint16"
            | "uint32"
            | "uint64"
            | "uint128"
            | "usize"
            | "float32"
            | "float64"
            | "float"
            | "bool"
            | "char"
    )
}

fn matching_brace(source: &str, opening: usize) -> Result<usize, String> {
    let mut depth = 0;
    for (offset, character) in source[opening..].char_indices() {
        match character {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(opening + offset);
                }
            }
            _ => {}
        }
    }
    Err("main body is not closed".to_string())
}
