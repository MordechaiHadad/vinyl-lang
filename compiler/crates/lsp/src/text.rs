use std::collections::HashSet;

use line_index::LineIndex;
use tower_lsp::lsp_types::Range;

use crate::position::position_at;

pub(crate) fn word_prefix(source: &str, offset: usize) -> String {
    let before = &source[..offset.min(source.len())];
    before
        .rsplit(|character: char| !character.is_alphanumeric() && character != '_')
        .next()
        .unwrap_or_default()
        .to_string()
}

pub(crate) fn detect_import_prefix(source: &str, offset: usize) -> Option<(usize, String)> {
    let before = &source[..offset.min(source.len())];
    let line_start = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line_prefix = &before[line_start..];
    let after_import = line_prefix.strip_prefix("import ").unwrap_or(line_prefix);

    let segments: Vec<&str> = after_import.split("::").collect();

    let mut prefix_count = 0;
    for segment in &segments {
        match *segment {
            "parent" | "self" => prefix_count += 1,
            _ => break,
        }
    }

    if prefix_count == 0 || segments.len() - prefix_count > 1 {
        return None;
    }

    let partial = if prefix_count >= segments.len() {
        String::new()
    } else {
        segments[prefix_count].to_string()
    };

    Some((prefix_count, partial))
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
    if word.is_empty() { None } else { Some(word.to_string()) }
}

pub(crate) fn module_ref_prefix(source: &str, offset: usize) -> Option<(String, String)> {
    let offset = offset.min(source.len());
    let before = &source[..offset];
    let bytes = source.as_bytes();

    let colon_at = before.rfind("::")
        .or_else(|| {
            if offset >= 1 && offset < source.len() && bytes[offset - 1] == b':' && bytes[offset] == b':' {
                Some(offset - 1)
            } else {
                None
            }
        })
        .or_else(|| {
            if offset + 1 < source.len() && &source[offset..offset + 2] == "::" {
                Some(offset)
            } else {
                None
            }
        })?;

    let before_colon = &source[..colon_at];
    let module_name = before_colon
        .rsplit(|c: char| !c.is_alphanumeric() && c != '_')
        .next()
        .unwrap_or("")
        .to_string();
    if module_name.is_empty() {
        return None;
    }
    let after_colon = &source[colon_at + 2..];
    let partial = after_colon
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .next()
        .unwrap_or("")
        .to_string();
    Some((module_name, partial))
}

pub(crate) fn extract_type_from_span(
    source: &str,
    offset: usize,
    length: usize,
    is_let: bool,
) -> Option<String> {
    let text = &source[offset..(offset + length)];
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
