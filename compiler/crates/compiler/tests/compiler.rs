use std::{fs, path::PathBuf};

use vinyl_compiler::compile_entry;

fn script_project(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = std::env::temp_dir().join(format!("vinyl_compiler_script_{name}"));
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
    let root = std::env::temp_dir().join(format!("vinyl_compiler_test_{name}"));
    let _ = fs::remove_dir_all(&root);
    for (file, source) in files {
        let path = root.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, source).unwrap();
    }
    fs::write(root.join("vinyl.toml"), "").unwrap();
    root
}

#[test]
fn compiles_public_import() {
    let root = project(
        "public_import",
        &[
            ("src/main.vn", "import math; fn main() { math::answer() }"),
            ("src/math.vn", "public fn answer(): int { 42 }"),
        ],
    );
    let result = compile_entry(&root.join("src/main.vn"), Some(&root));
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn main_must_return_unit() {
    let root = script_project("main_return_type", &[("main.vn", "fn main(): int { 0 }\n")]);
    let errors = compile_entry(&root.join("main.vn"), None).unwrap_err();
    let message = errors
        .iter()
        .map(ToString::to_string)
        .find(|message| message.contains("main function must return `unit`"))
        .expect("main return type diagnostic");
    assert!(message.contains("main function must return `unit`"));
}

#[test]
fn missing_module_import_is_a_type_diagnostic() {
    let root = script_project(
        "missing_import_diagnostic",
        &[
            ("main.vn", "fn main() { math::double() }\n"),
            ("math.vn", "public fn double(): int { 2 }\n"),
        ],
    );
    let errors = compile_entry(&root.join("main.vn"), None).unwrap_err();
    let message = errors
        .iter()
        .map(ToString::to_string)
        .find(|message| message.contains("missing import"))
        .expect("missing import diagnostic");
    assert!(message.contains("import parent::math"), "{message}");
}

#[test]
fn rejects_private_import() {
    let root = project(
        "private_import",
        &[
            ("src/main.vn", "import math; fn main() { math::answer() }"),
            ("src/math.vn", "fn answer(): int { 42 }"),
        ],
    );
    let result = compile_entry(&root.join("src/main.vn"), Some(&root));
    assert!(result.is_err());
}

#[test]
fn import_not_found_errors() {
    let root = project(
        "import_not_found",
        &[("src/main.vn", "import math; fn main() { 0 }")],
    );
    let result = compile_entry(&root.join("src/main.vn"), Some(&root));
    assert!(result.is_err());
}

#[test]
fn nested_module_import() {
    let root = project(
        "nested_import",
        &[
            (
                "src/main.vn",
                "import utils::format; fn main() { format::greet() }",
            ),
            (
                "src/utils/format.vn",
                "public fn greet(): string { \"hi\" }",
            ),
        ],
    );
    let result = compile_entry(&root.join("src/main.vn"), Some(&root));
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn entry_without_main_or_lib() {
    let root = project("no_entry", &[("src/foo.vn", "fn foo(): int { 1 }")]);
    let result = compile_entry(&root, Some(&root));
    assert!(result.is_err());
}

#[test]
fn compiles_script_project_with_import() {
    let root = script_project(
        "script_import",
        &[
            ("main.vn", "import math; fn main() { math::double(21) }"),
            ("math.vn", "public fn double(n: int): int { n * 2 }"),
        ],
    );
    let result = compile_entry(&root.join("main.vn"), None);
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn compiles_nested_module_symbols() {
    let root = project(
        "nested_module_symbols",
        &[
            (
                "src/main.vn",
                "import parent::nested::math; fn main() { let point = parent::nested::math::Point { x: 69, y: 69 }; parent::nested::math::double(69) }",
            ),
            (
                "src/nested/math.vn",
                "public fn double(n: int): int { n * 2 } public struct Point { public x: int, public y: int }",
            ),
        ],
    );
    let result = compile_entry(&root.join("src/main.vn"), Some(&root));
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn compiles_deep_nested_module_symbols() {
    let root = project(
        "deep_nested_module_symbols",
        &[
            (
                "src/main.vn",
                "import parent::mod1::mod2::mod3::infinite; fn main() { let value = parent::mod1::mod2::mod3::infinite::Enum::Variant; match value { parent::mod1::mod2::mod3::infinite::Enum::Variant => unit, _ => unit }; let point = parent::mod1::mod2::mod3::infinite::Struct { x: 69, y: 69 }; parent::mod1::mod2::mod3::infinite::function(69) }",
            ),
            (
                "src/mod1/mod2/mod3/infinite.vn",
                "public fn function(n: int): int { n * 2 } public struct Struct { public x: int, public y: int } public enum Enum { public Variant }",
            ),
        ],
    );
    let result = compile_entry(&root.join("src/main.vn"), Some(&root));
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn compiles_manifest_via_detect() {
    let root = project(
        "manifest_detect",
        &[
            ("src/main.vn", "import math; fn main() { math::answer() }"),
            ("src/math.vn", "public fn answer(): int { 42 }"),
        ],
    );
    let result = compile_entry(&root.join("src/main.vn"), None);
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn resolves_directory_module() {
    let root = project(
        "directory_module",
        &[
            ("src/main.vn", "import math; fn main() { math::answer() }"),
            ("src/math/math.vn", "public fn answer(): int { 42 }"),
        ],
    );
    let result = compile_entry(&root.join("src/main.vn"), Some(&root));
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn script_self_prefix_errors() {
    let root = script_project(
        "script_self_errors",
        &[("main.vn", "import self::helper; fn main() { 0 }")],
    );
    let result = compile_entry(&root.join("main.vn"), None);
    assert!(result.is_err(), "self:: should error in imports");
}

#[test]
fn script_parent_prefix_same_dir() {
    let root = script_project(
        "script_parent_same_dir",
        &[
            (
                "sub/main.vn",
                "import parent::helper; fn main() { helper::answer() }",
            ),
            ("sub/helper.vn", "public fn answer(): int { 42 }"),
        ],
    );
    let result = compile_entry(&root.join("sub/main.vn"), None);
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn script_parent_parent_prefix() {
    let root = script_project(
        "script_parent_parent",
        &[
            (
                "sub/main.vn",
                "import parent::parent::helper; fn main() { helper::answer() }",
            ),
            ("helper.vn", "public fn answer(): int { 42 }"),
        ],
    );
    let result = compile_entry(&root.join("sub/main.vn"), None);
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn script_package_prefix_rejected() {
    let root = script_project(
        "script_package_rejected",
        &[
            ("main.vn", "import package::helper; fn main() { 0 }"),
            ("helper.vn", "public fn answer(): int { 42 }"),
        ],
    );
    let result = compile_entry(&root.join("main.vn"), None);
    assert!(
        result.is_err(),
        "package:: should be rejected in script mode"
    );
}

#[test]
fn manifest_self_prefix_errors() {
    let root = project(
        "manifest_self_errors",
        &[("src/main.vn", "import self::helper; fn main() { 0 }")],
    );
    let result = compile_entry(&root.join("src/main.vn"), Some(&root));
    assert!(result.is_err(), "self:: should error in imports");
}

#[test]
fn manifest_parent_prefix_same_dir() {
    let root = project(
        "manifest_parent_same_dir",
        &[
            (
                "src/sub/main.vn",
                "import parent::helper; fn main() { helper::answer() }",
            ),
            ("src/sub/helper.vn", "public fn answer(): int { 42 }"),
        ],
    );
    let result = compile_entry(&root.join("src/sub/main.vn"), Some(&root));
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn manifest_parent_parent_prefix() {
    let root = project(
        "manifest_parent_parent",
        &[
            (
                "src/sub/main.vn",
                "import parent::parent::helper; fn main() { helper::answer() }",
            ),
            ("src/helper.vn", "public fn answer(): int { 42 }"),
        ],
    );
    let result = compile_entry(&root.join("src/sub/main.vn"), Some(&root));
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn manifest_package_prefix() {
    let root = project(
        "manifest_package_prefix",
        &[
            (
                "src/main.vn",
                "import package::helper; fn main() { helper::answer() }",
            ),
            ("src/helper.vn", "public fn answer(): int { 42 }"),
        ],
    );
    let result = compile_entry(&root.join("src/main.vn"), Some(&root));
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn symbol_import_function_bare_call() {
    let root = project(
        "symbol_import_function",
        &[
            (
                "src/main.vn",
                "import math::double; fn main() { double(21) }",
            ),
            ("src/math.vn", "public fn double(n: int): int { n * 2 }"),
        ],
    );
    let result = compile_entry(&root.join("src/main.vn"), Some(&root));
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn symbol_import_private_errors() {
    let root = project(
        "symbol_import_private",
        &[
            ("src/main.vn", "import math::hidden; fn main() { hidden() }"),
            ("src/math.vn", "fn hidden(): int { 42 }"),
        ],
    );
    let result = compile_entry(&root.join("src/main.vn"), Some(&root));
    assert!(result.is_err(), "importing a private symbol should error");
}

#[test]
fn wildcard_import_bare_call() {
    let root = project(
        "wildcard_import",
        &[
            ("src/main.vn", "import math::*; fn main() { answer() }"),
            ("src/math.vn", "public fn answer(): int { 42 }"),
        ],
    );
    let result = compile_entry(&root.join("src/main.vn"), Some(&root));
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn grouped_symbol_import_bare_calls() {
    let root = project(
        "grouped_symbol_import",
        &[
            (
                "src/main.vn",
                "import math::{double, triple}; fn main() { double(2) + triple(2) }",
            ),
            (
                "src/math.vn",
                "public fn double(n: int): int { n * 2 } public fn triple(n: int): int { n * 3 }",
            ),
        ],
    );
    let result = compile_entry(&root.join("src/main.vn"), Some(&root));
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn symbol_import_conflict_errors() {
    let root = project(
        "symbol_import_conflict",
        &[
            (
                "src/main.vn",
                "import a::foo; import b::foo; fn main() { foo() }",
            ),
            ("src/a.vn", "public fn foo(): int { 1 }"),
            ("src/b.vn", "public fn foo(): int { 2 }"),
        ],
    );
    let result = compile_entry(&root.join("src/main.vn"), Some(&root));
    assert!(result.is_err(), "conflicting symbol imports should error");
}

#[test]
fn scoped_type_public_field_access() {
    let root = project(
        "scoped_type_public_field",
        &[
            (
                "src/main.vn",
                "import math; fn area(s: math::Shape): float64 { s.radius }",
            ),
            (
                "src/math.vn",
                "public struct Shape { public radius: float64 }",
            ),
        ],
    );
    let result = compile_entry(&root.join("src/main.vn"), Some(&root));
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn scoped_type_private_field_errors() {
    let root = project(
        "scoped_type_private_field",
        &[
            (
                "src/main.vn",
                "import math; fn area(s: math::Shape): float64 { s.radius }",
            ),
            ("src/math.vn", "public struct Shape { radius: float64 }"),
        ],
    );
    let result = compile_entry(&root.join("src/main.vn"), Some(&root));
    assert!(
        result.is_err(),
        "cross-module private field access should error"
    );
}

#[test]
fn scoped_private_type_errors() {
    let root = project(
        "scoped_private_type",
        &[
            (
                "src/main.vn",
                "import math; fn f(s: math::Hidden): int32 { 0 }",
            ),
            ("src/math.vn", "struct Hidden {}"),
        ],
    );
    let result = compile_entry(&root.join("src/main.vn"), Some(&root));
    assert!(
        result.is_err(),
        "referencing a private type from another module should error"
    );
}

#[test]
fn module_qualified_enum_variant_construction() {
    let root = project(
        "module_qualified_variant",
        &[
            (
                "src/main.vn",
                "import math; fn main(): unit { let s = math::Shape::Circle; }",
            ),
            (
                "src/math.vn",
                "public enum Shape { public Circle, Square(float64) }",
            ),
        ],
    );
    let result = compile_entry(&root.join("src/main.vn"), Some(&root));
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn module_qualified_private_variant_errors() {
    let root = project(
        "module_qualified_private_variant",
        &[
            (
                "src/main.vn",
                "import math; fn main(): unit { let s = math::Shape::Circle; }",
            ),
            ("src/math.vn", "public enum Shape { Circle }"),
        ],
    );
    let result = compile_entry(&root.join("src/main.vn"), Some(&root));
    assert!(result.is_err(), "cross-module private variant should error");
}

#[test]
fn imported_type_public_field_access() {
    let root = project(
        "imported_type_public_field",
        &[
            (
                "src/main.vn",
                "import math; fn main() { math::make_point().x }",
            ),
            (
                "src/math.vn",
                "public struct Point { public x: int32 } public fn make_point(): Point { Point { x: 1 } }",
            ),
        ],
    );
    let result = compile_entry(&root.join("src/main.vn"), Some(&root));
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn imported_type_private_field_errors() {
    let root = project(
        "imported_type_private_field",
        &[
            (
                "src/main.vn",
                "import math; fn main() { math::make_point().x }",
            ),
            (
                "src/math.vn",
                "public struct Point { x: int32 } public fn make_point(): Point { Point { x: 1 } }",
            ),
        ],
    );
    let result = compile_entry(&root.join("src/main.vn"), Some(&root));
    assert!(
        result.is_err(),
        "cross-module private field access should error"
    );
}
