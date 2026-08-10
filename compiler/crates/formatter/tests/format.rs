use std::{fs, path::PathBuf};

use vinyl_formatter::{
    FormatterConfig, format_project, format_range, format_source, format_source_with_config,
};

fn script_project(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = std::env::temp_dir().join(format!("vinyl_formatter_script_{name}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    for (file, source) in files {
        let path = root.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, source).unwrap();
    }
    root
}

fn project(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = std::env::temp_dir().join(format!("vinyl_formatter_test_{name}"));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    for (file, source) in files {
        let path = root.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, source).unwrap();
    }
    fs::write(root.join("vinyl.toml"), "").unwrap();
    root
}

#[test]
fn formats_function_def() {
    let input = "fn    add ( a : int , b : int ) : int {  return  a + b ; }";
    let expected = "fn add(a: int, b: int): int {\n    return a + b;\n}";
    assert_eq!(format_source(input).unwrap(), expected);
}

#[test]
fn formats_public_fn() {
    let input = "public fn greet (name : string ): string { return \"hi\" +name ; }";
    let expected = "public fn greet(name: string): string {\n    return \"hi\" + name;\n}";
    assert_eq!(format_source(input).unwrap(), expected);
}

#[test]
fn formats_if_else() {
    let input = "fn max(a:int,b:int):int{if a>b{return a;}else{return b;}}";
    let expected = "fn max(a: int, b: int): int {\n    if a > b {\n        return a;\n    } else {\n        return b;\n    }\n}";
    assert_eq!(format_source(input).unwrap(), expected);
}

#[test]
fn match_arm_guard_gets_spaces() {
    let input = "fn classify_signed(n:int):int{match n{0=>0\nx if x<0=>1\n_=>2}}";
    let expected = "fn classify_signed(n: int): int {\n    match n {\n        0 => 0\n        x if x < 0 => 1\n        _ => 2\n    }\n}";
    assert_eq!(format_source(input).unwrap(), expected);
}

#[test]
fn formats_import() {
    let input = "import   math ;";
    let expected = "import math;";
    assert_eq!(format_source(input).unwrap(), expected);
}

#[test]
fn formats_type_alias() {
    let input = "type    Point  =  ( float64 , float64 ) ;";
    let expected = "type Point = (float64, float64);";
    assert_eq!(format_source(input).unwrap(), expected);
}

#[test]
fn formats_public_type_alias() {
    let input = "public type Bytes = [int32; 4];";
    let expected = "public type Bytes = [int32;4];";
    assert_eq!(format_source(input).unwrap(), expected);
}

#[test]
fn formats_struct_def_keyword_once() {
    let input = "struct Point { x: int32, y: int32 }";
    let expected = "struct Point {\n    x:int32,\n    y:int32\n}";
    assert_eq!(format_source(input).unwrap(), expected);
}

#[test]
fn formats_tuple_def_keyword_once() {
    let input = "tuple Pair (int32, float64)";
    let expected = "tuple Pair (int32, float64)";
    assert_eq!(format_source(input).unwrap(), expected);
}

#[test]
fn formats_enum_def_keyword_once() {
    let input = "enum Color { Red, Green(int32) }";
    let expected = "enum Color {\n    Red,\n    Green(int32)\n}";
    assert_eq!(format_source(input).unwrap(), expected);
}

#[test]
fn preserves_blank_lines_in_block() {
    let input = "fn f() {\n    let mut x = 10;\n\n    x = 69;\n}";
    let expected = "fn f() {\n    let mut x = 10;\n\n    x = 69;\n}";
    assert_eq!(format_source(input).unwrap(), expected);
}

#[test]
fn preserves_multiple_blank_lines_as_single_in_block() {
    let input = "fn f() {\n    let mut x = 10;\n\n\n    x = 69;\n}";
    let expected = "fn f() {\n    let mut x = 10;\n\n    x = 69;\n}";
    assert_eq!(format_source(input).unwrap(), expected);
}

#[test]
fn no_extra_blank_line_without_one_in_block() {
    let input = "fn f() {\n    let mut x = 10;\n    x = 69;\n}";
    let result = format_source(input).unwrap();
    let lines: Vec<&str> = result.lines().collect();
    assert!(lines.contains(&"    let mut x = 10;"));
    assert!(lines.contains(&"    x = 69;"));
    let x_idx = lines.iter().position(|l| l.contains("x = 10")).unwrap();
    let y_idx = lines.iter().position(|l| l.contains("x = 69")).unwrap();
    assert_eq!(y_idx - x_idx, 1, "should be adjacent without blank line");
}

#[test]
fn empty_source() {
    assert_eq!(format_source("").unwrap(), "");
}

#[test]
fn preserves_crlf_line_endings() {
    let input = "fn f(): int {\r\n    return 1;\r\n}\r\n";
    assert_eq!(format_source(input).unwrap(), input);
}

#[test]
fn formats_crlf_without_changing_line_endings() {
    let input = "fn   f(): int {\r\nreturn 1;\r\n}";
    let expected = "fn f(): int {\r\n    return 1;\r\n}";
    assert_eq!(format_source(input).unwrap(), expected);
}

#[test]
fn configurable_indent_width() {
    let input = "fn f(): int {\nreturn 1;\n}";
    let config = FormatterConfig { indent_width: 2 };
    let result = format_source_with_config(input, &config).unwrap();
    assert!(
        result.contains("  return 1;"),
        "expected 2-space indent, got:\n{result}"
    );
}

#[test]
fn format_range_delegates_to_full_format() {
    let input = "fn    add ( a : int ) : int {  return  a ; }";
    let expected = "fn add(a: int): int {\n    return a;\n}";
    let range = 0..input.len();
    assert_eq!(
        format_range(input, &FormatterConfig::default(), range).unwrap(),
        expected
    );
}

#[test]
fn preserves_blank_line_above_attribute() {
    let input = "fn a(): int {\n    1\n}\n\n@doc(\"x\")\nfn b(): int {\n    2\n}";
    let expected = "fn a(): int {\n    1\n}\n\n@doc(\"x\")\nfn b(): int {\n    2\n}";
    assert_eq!(format_source(input).unwrap(), expected);
}

#[test]
fn collapses_multiple_blank_lines_above_attribute_to_one() {
    let input = "fn a(): int {\n    1\n}\n\n\n\n@doc(\"x\")\nfn b(): int {\n    2\n}";
    let expected = "fn a(): int {\n    1\n}\n\n@doc(\"x\")\nfn b(): int {\n    2\n}";
    assert_eq!(format_source(input).unwrap(), expected);
}

#[test]
fn preserves_blank_line_above_public_definition() {
    let input = "fn a(): int {\n    1\n}\n\npublic fn b(): int {\n    2\n}";
    let expected = "fn a(): int {\n    1\n}\n\npublic fn b(): int {\n    2\n}";
    assert_eq!(format_source(input).unwrap(), expected);
}

#[test]
fn format_project_empty_src() {
    let root = project("empty", &[]);
    fs::create_dir_all(root.join("src")).unwrap();
    let result = format_project(&root);
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn format_project_formats_all_files() {
    let root = project(
        "all_files",
        &[
            ("src/a.vn", "fn   a(): int { 1 }"),
            ("src/b.vn", "fn   b(): int { 2 }"),
        ],
    );
    format_project(&root).unwrap();
    let a = fs::read_to_string(root.join("src/a.vn")).unwrap();
    let b = fs::read_to_string(root.join("src/b.vn")).unwrap();
    assert_eq!(a, "fn a(): int {\n    1\n}");
    assert_eq!(b, "fn b(): int {\n    2\n}");
}

#[test]
fn format_script_project() {
    let root = script_project(
        "script",
        &[
            ("main.vn", "fn   main(): int { 1 }"),
            ("utils.vn", "public fn   helper(): int { 2 }"),
        ],
    );
    format_project(&root).unwrap();
    let main = fs::read_to_string(root.join("main.vn")).unwrap();
    let utils = fs::read_to_string(root.join("utils.vn")).unwrap();
    assert_eq!(main, "fn main(): int {\n    1\n}");
    assert_eq!(utils, "public fn helper(): int {\n    2\n}");
}
