/// Absolute path to the compiler-shipped standard library source. The resolver
/// registers `std` as a regular on-disk module pointing at this file, so
/// `import std;` flows through the normal module machinery.
pub const STD_SOURCE_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/std.vn");
