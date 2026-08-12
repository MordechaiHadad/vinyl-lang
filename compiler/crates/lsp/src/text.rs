use std::collections::HashSet;

use line_index::LineIndex;
use tower_lsp::lsp_types::Range;

use crate::position::position_at;

pub(crate) fn name_range(source: &str, span: (usize, usize), name: &str) -> (usize, usize) {
    name_span(source, span, name).unwrap_or(span)
}

pub(crate) fn name_span(source: &str, span: (usize, usize), name: &str) -> Option<(usize, usize)> {
    let (start, end) = span;
    let text = source.get(start..end)?;
    find_identifier(text, name).map(|relative| (start + relative, start + relative + name.len()))
}

pub(crate) enum ModulePathContext {
    ImportPath {
        segments: Vec<String>,
        partial: String,
    },
    ImportSymbol {
        module_name: String,
        partial: String,
    },
    ModuleRef {
        module_name: String,
        partial: String,
        scope_qualified: bool,
    },
}

pub(crate) fn module_path_context(source: &str, offset: usize) -> Option<ModulePathContext> {
    let offset = offset.min(source.len());
    let line_start = source[..offset].rfind('\n').map_or(0, |index| index + 1);
    let line = &source[line_start..offset];

    if let Some(after_import) = line.trim_start().strip_prefix("import ") {
        let path_end = after_import
            .find(|character: char| character == ';' || character.is_whitespace())
            .unwrap_or(after_import.len());
        let path = &after_import[..path_end];
        let segments: Vec<&str> = path.split("::").collect();
        let prefix_count = segments
            .iter()
            .take_while(|segment| matches!(**segment, "parent" | "self" | "package"))
            .count();
        if segments.len() - prefix_count <= 1 {
            return Some(ModulePathContext::ImportPath {
                segments: segments[..prefix_count]
                    .iter()
                    .map(|segment| segment.to_string())
                    .collect(),
                partial: segments.get(prefix_count).unwrap_or(&"").to_string(),
            });
        }
        let module_name = segments.get(prefix_count)?.to_string();
        if module_name.is_empty() {
            return None;
        }
        return Some(ModulePathContext::ImportSymbol {
            module_name,
            partial: segments[prefix_count + 1..].join("::"),
        });
    }

    let (module_name, partial) = module_ref_prefix(source, offset)?;
    Some(ModulePathContext::ModuleRef {
        module_name,
        partial,
        scope_qualified: is_scope_qualified(&source[line_start..offset]),
    })
}

fn is_scope_qualified(line: &str) -> bool {
    let Some(colon_at) = line.rfind("::") else {
        return false;
    };
    let mut chunks = line[..colon_at]
        .rsplit(|character: char| !character.is_alphanumeric() && character != '_')
        .filter(|chunk| !chunk.is_empty());
    let module_name = chunks.next().unwrap_or_default();
    if matches!(module_name, "parent" | "self" | "package") {
        return true;
    }
    matches!(chunks.next(), Some("parent" | "self" | "package"))
}

fn find_identifier(text: &str, name: &str) -> Option<usize> {
    let name_len = name.len();
    let is_ident = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_';
    text.match_indices(name).find_map(|(index, _)| {
        let before = text.as_bytes().get(index.wrapping_sub(1)).copied();
        let after = text.as_bytes().get(index + name_len).copied();
        let before_ok = before.is_none_or(|byte| !is_ident(byte));
        let after_ok = after.is_none_or(|byte| !is_ident(byte));
        (before_ok && after_ok).then_some(index)
    })
}

pub(crate) fn word_prefix(source: &str, offset: usize) -> String {
    let before = &source[..offset.min(source.len())];
    before
        .rsplit(|character: char| !character.is_alphanumeric() && character != '_')
        .next()
        .unwrap_or_default()
        .to_string()
}

pub(crate) fn word_before_colon(source: &str, offset: usize) -> Option<String> {
    let offset = offset.min(source.len());
    if offset == 0 {
        return None;
    }
    let before = &source[..offset];
    let bytes = before.as_bytes();
    if bytes[offset - 1] != b':' {
        return None;
    }
    if offset >= 2 && bytes[offset - 2] == b':' {
        return None;
    }
    let word_start = before[..offset - 1]
        .rfind(|c: char| !c.is_alphanumeric() && c != '_')
        .map(|i| i + 1)
        .unwrap_or(0);
    let word = &before[word_start..offset - 1];
    if word.is_empty() {
        None
    } else {
        Some(word.to_string())
    }
}

pub(crate) fn module_ref_prefix(source: &str, offset: usize) -> Option<(String, String)> {
    let offset = offset.min(source.len());
    let line_start = source[..offset].rfind('\n').map_or(0, |index| index + 1);
    let line = &source[line_start..];
    let relative_offset = offset - line_start;
    let before = &line[..relative_offset];
    let colon_at = before.rfind("::").or_else(|| {
        if relative_offset > 0
            && line.as_bytes()[relative_offset - 1] == b':'
            && line[relative_offset..].starts_with(':')
        {
            Some(relative_offset - 1)
        } else {
            line[relative_offset..]
                .starts_with("::")
                .then_some(relative_offset)
        }
    })?;
    let symbol_start = colon_at + 2;
    let symbol_end = line[symbol_start..]
        .find(|character: char| !character.is_alphanumeric() && character != '_')
        .map_or(line.len(), |index| symbol_start + index);
    let symbol_name = &line[symbol_start..symbol_end];
    if relative_offset < symbol_start
        || !line[symbol_start..relative_offset]
            .chars()
            .all(|character| character.is_alphanumeric() || character == '_')
    {
        return None;
    }

    let before_colon = &line[..colon_at];
    let module_name = before_colon
        .rsplit(|c: char| !c.is_alphanumeric() && c != '_')
        .next()
        .unwrap_or("")
        .to_string();
    if module_name.is_empty() {
        return None;
    }
    Some((module_name, symbol_name.to_string()))
}

pub(crate) fn extract_type_from_span(
    source: &str,
    offset: usize,
    length: usize,
    is_let: bool,
) -> Option<String> {
    let end = offset.checked_add(length)?;
    if !source.is_char_boundary(offset) || !source.is_char_boundary(end) {
        return None;
    }
    let text = source.get(offset..end)?;
    let type_text = if is_let {
        let colon = text.find(':')?;
        let after_colon = &text[colon + 1..];
        let eq = after_colon.find('=').unwrap_or(after_colon.len());
        after_colon[..eq].trim().to_string()
    } else {
        text.split(':').nth(1)?.trim().to_string()
    };
    if type_text.is_empty() {
        None
    } else {
        Some(type_text)
    }
}

pub(crate) fn import_edit_range(line_index: &LineIndex, source: &str) -> Range {
    let mut offset = 0usize;
    for line in source.lines() {
        if line.trim_start().starts_with("import ") {
            offset += line.len() + 1;
        } else {
            break;
        }
    }
    let pos = position_at(line_index, offset.min(source.len()));
    Range::new(pos, pos)
}

pub(crate) fn current_imports(source: &str) -> HashSet<String> {
    source
        .lines()
        .filter_map(|line| line.trim().strip_prefix("import "))
        .map(|s| s.trim_end_matches(';').trim().to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::extract_type_from_span;

    #[test]
    fn stale_type_span_returns_none() {
        assert_eq!(extract_type_from_span("fn main() {}", 71, 4, false), None);
    }

    #[test]
    fn non_character_boundary_returns_none() {
        assert_eq!(extract_type_from_span("é: int", 1, 3, false), None);
    }
}
