//! 源文档字节 Span ↔ 行列（LSP 用 UTF-16 列）。

use crate::ast::Span;

/// 每行起始的 UTF-8 字节下标（含 `source.len()` 哨兵）。
pub fn line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (i, b) in source.as_bytes().iter().enumerate() {
        if *b == b'\n' {
            starts.push(i + 1);
        }
    }
    if starts.last().copied() != Some(source.len()) {
        starts.push(source.len());
    }
    starts
}

/// UTF-8 字节偏移 → `(line, character)`，character 为 UTF-16 code unit 下标（0-based）。
pub fn offset_to_position(source: &str, starts: &[usize], offset: usize) -> (u32, u32) {
    let offset = offset.min(source.len());
    let line = match starts.binary_search(&offset) {
        Ok(i) => {
            // 恰好落在某行首；若是哨兵则落在最后一行末
            if i + 1 == starts.len() {
                i.saturating_sub(1)
            } else {
                i
            }
        }
        Err(i) => i.saturating_sub(1),
    };
    let line_start = starts.get(line).copied().unwrap_or(0);
    let slice = &source[line_start..offset];
    let character = utf16_len(slice);
    (line as u32, character)
}

/// `Span` → LSP 半开 Range：`(start_line, start_col, end_line, end_col)`。
pub fn span_to_range(source: &str, span: Span) -> (u32, u32, u32, u32) {
    let starts = line_starts(source);
    let (sl, sc) = offset_to_position(source, &starts, span.start);
    let (el, ec) = offset_to_position(source, &starts, span.end);
    (sl, sc, el, ec)
}

fn utf16_len(s: &str) -> u32 {
    s.chars().map(|c| c.len_utf16() as u32).sum()
}

/// 字节偏移是否落在 Span 半开区间内。
pub fn offset_in_span(offset: usize, span: Span) -> bool {
    offset >= span.start && offset < span.end
}

/// LSP `(line, character)`（UTF-16 列）→ UTF-8 字节偏移。
pub fn position_to_offset(source: &str, starts: &[usize], line: u32, character: u32) -> Option<usize> {
    let line = line as usize;
    let line_start = *starts.get(line)?;
    let line_end = starts.get(line + 1).copied().unwrap_or(source.len());
    let line_text = source.get(line_start..line_end)?;
    let mut utf16_i = 0u32;
    for (byte_rel, ch) in line_text.char_indices() {
        if utf16_i >= character {
            return Some(line_start + byte_rel);
        }
        utf16_i += ch.len_utf16() as u32;
    }
    Some(
        line_end
            .saturating_sub(if line_text.ends_with('\n') { 1 } else { 0 })
            .max(line_start),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_positions() {
        let src = "ab\ncd";
        let starts = line_starts(src);
        assert_eq!(offset_to_position(src, &starts, 0), (0, 0));
        assert_eq!(offset_to_position(src, &starts, 2), (0, 2));
        assert_eq!(offset_to_position(src, &starts, 3), (1, 0));
        assert_eq!(span_to_range(src, Span::new(3, 5)), (1, 0, 1, 2));
    }

    #[test]
    fn cjk_utf16_columns() {
        let src = "甲乙\n丙";
        let starts = line_starts(src);
        // 甲 = 1 UTF-16 unit
        assert_eq!(offset_to_position(src, &starts, "甲".len()), (0, 1));
        assert_eq!(offset_to_position(src, &starts, "甲乙".len()), (0, 2));
    }
}
