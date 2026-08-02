use line_index::{LineCol, LineIndex, TextSize, WideEncoding, WideLineCol};
use tower_lsp::lsp_types::{Position, Range};

pub(crate) fn offset_at(line_index: &LineIndex, position: Position) -> usize {
    let wide_col = WideLineCol {
        line: position.line,
        col: position.character,
    };
    let line_col = line_index
        .to_utf8(WideEncoding::Utf16, wide_col)
        .unwrap_or(LineCol {
            line: position.line,
            col: 0,
        });
    line_index.offset(line_col).map(TextSize::into).unwrap_or(0)
}

pub(crate) fn span_range(line_index: &LineIndex, offset: usize, length: usize) -> Range {
    Range::new(
        position_at(line_index, offset),
        position_at(line_index, offset + length),
    )
}

pub(crate) fn full_range(line_index: &LineIndex) -> Range {
    Range::new(
        Position::new(0, 0),
        position_at(line_index, line_index.len().into()),
    )
}

pub(crate) fn position_at(line_index: &LineIndex, offset: usize) -> Position {
    let offset = TextSize::try_from(offset).unwrap_or(line_index.len());
    let line_col = line_index.line_col(offset.min(line_index.len()));
    let wide_col = line_index
        .to_wide(WideEncoding::Utf16, line_col)
        .unwrap_or(WideLineCol {
            line: line_col.line,
            col: 0,
        });
    Position::new(wide_col.line, wide_col.col)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_use_utf16_columns() {
        let source = "😀value";
        let line_index = LineIndex::new(source);
        assert_eq!(offset_at(&line_index, Position::new(0, 2)), 4);
        assert_eq!(position_at(&line_index, 4), Position::new(0, 2));
    }
}
