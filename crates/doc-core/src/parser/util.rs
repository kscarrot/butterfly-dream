//! 解析器用的小段文本工具（避免各文件各写一份）。

/// `i` 处的 Unicode 标量值。
pub(super) fn char_at(s: &str, i: usize) -> Option<char> {
    s.get(i..)?.chars().next()
}

/// 从字节下标 `i` 前进一个 Unicode 标量值。
pub(super) fn next_char(s: &str, i: usize) -> usize {
    char_at(s, i).map(|c| i + c.len_utf8()).unwrap_or(s.len())
}

/// `i` 是否位于行首（文首或前一字节为 `\n`）。
pub(super) fn at_bol(s: &str, i: usize) -> bool {
    i == 0 || s.as_bytes().get(i - 1) == Some(&b'\n')
}

/// 从 `start` 到本行末的切片（不含 `\n`；`lines` 会剥掉 `\r`）。
pub(super) fn same_line(s: &str, start: usize) -> &str {
    s.get(start..).and_then(|r| r.lines().next()).unwrap_or("")
}

/// 行首空白字节数。
pub(super) fn leading_indent(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// CommonMark 风格：前导空格 ≤ 3 才算「行首块」候选。
pub(super) fn indent_ok_cm(line: &str) -> bool {
    leading_indent(line) <= 3
}

/// 若 `end` 恰指向 `\n`，把该换行算进半开区间终点。
pub(super) fn include_trailing_nl(s: &str, end: usize) -> usize {
    if end < s.len() && s.as_bytes()[end] == b'\n' {
        end + 1
    } else {
        end
    }
}

/// 剥开头的 `\r\n` 或 `\n`；非行尾则 `None`。
pub(super) fn strip_eol(s: &str) -> Option<&str> {
    s.strip_prefix("\r\n").or_else(|| s.strip_prefix('\n'))
}

/// 空串视为已在 EOF；否则须为行尾。
pub(super) fn strip_eol_or_eof(s: &str) -> Option<&str> {
    if s.is_empty() {
        Some("")
    } else {
        strip_eol(s)
    }
}

/// 锚点 / class 等标识符允许的字符：`[A-Za-z0-9_-]`。
pub(super) fn is_id_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}
