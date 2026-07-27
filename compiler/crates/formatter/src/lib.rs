pub mod error;
mod tree;

pub use error::FormatError;

use std::path::Path;

pub struct FormatterConfig {
    pub indent_width: usize,
}

impl Default for FormatterConfig {
    fn default() -> Self {
        FormatterConfig { indent_width: 4 }
    }
}

pub fn format_source(source: &str) -> Result<String, FormatError> {
    tree::format_source(source)
}

pub fn format_source_with_config(
    source: &str,
    config: &FormatterConfig,
) -> Result<String, FormatError> {
    tree::format_source_with_config(source, config)
}

pub fn format_range(
    source: &str,
    config: &FormatterConfig,
    range: std::ops::Range<usize>,
) -> Result<String, FormatError> {
    tree::format_range(source, config, range)
}

pub fn format_path(path: &Path) -> Result<(), Vec<FormatError>> {
    let source = std::fs::read_to_string(path).map_err(|e| vec![FormatError::Io(e)])?;
    let formatted = format_source(&source).map_err(|e| vec![e])?;
    if formatted != source {
        std::fs::write(path, &formatted).map_err(|e| vec![FormatError::Io(e)])?;
    }
    Ok(())
}

pub fn format_project(source_root: &Path) -> Result<(), Vec<FormatError>> {
    let source_root = source_root
        .canonicalize()
        .map_err(|e| vec![FormatError::Io(e)])?;
    let resolver = vinyl_resolver::ModuleResolver::new(&source_root)
        .map_err(|e| vec![FormatError::Resolve(e)])?;
    let mut errors = Vec::new();
    for info in resolver.all_modules().values() {
        if let Err(e) = format_path(&info.file_path) {
            errors.extend(e);
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn formats_import() {
        let input = "import   math ;";
        let expected = "import math;";
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
}
