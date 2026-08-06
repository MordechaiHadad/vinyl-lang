mod common;

use vinyl_typecheck::module::ModuleTable;

fn positions(source: &str) -> std::collections::BTreeMap<usize, String> {
    let items = vinyl_parser::parse_and_lower(source).unwrap();
    let table = ModuleTable::new();
    let (result, _) =
        vinyl_typecheck::typeck_with_index(&items, source, "<test>", &table).unwrap();
    result.type_positions
}

#[test]
fn tuple_variant_sugar_types_recorded_at_source_offsets() {
    let source = "enum Shape { Circle(int), Rect(float, int) }";
    let positions = positions(source);

    let circle_int = source.find("Circle(int)").unwrap() + "Circle(".len();
    let rect_float = source.find("Rect(float, int)").unwrap() + "Rect(".len();
    let rect_int = rect_float + "float, ".len();

    assert_eq!(positions.get(&circle_int).map(String::as_str), Some("int"));
    assert_eq!(
        positions.get(&rect_float).map(String::as_str),
        Some("float")
    );
    assert_eq!(positions.get(&rect_int).map(String::as_str), Some("int"));
}

#[test]
fn tuple_struct_sugar_types_do_not_overflow_source() {
    let source = "tuple Point(int, uint)";
    let positions = positions(source);

    let point_int = source.find("Point(int, uint)").unwrap() + "Point(".len();
    let point_uint = point_int + "int, ".len();

    assert_eq!(positions.get(&point_int).map(String::as_str), Some("int"));
    assert_eq!(positions.get(&point_uint).map(String::as_str), Some("uint"));
}
