//! JMD 正文禁止的 Markdown 代码语法识别。
//!
//! - 行首围栏：`` ``` `` / `~~~`（缩进 ≤ 3）
//! - 行内反引号：任意长度开界定符，同行等长闭合
//!
//! 本模块只负责**匹配**（返回半开区间终点）；诊断与抹除由 `scan` 完成。

use super::util::{include_trailing_nl, indent_ok_cm, same_line};

/// 禁止代码的匹配结果；`end` 均为逻辑正文中的半开结束下标。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CodeHit {
    /// 已闭合；围栏时 `end` 含闭行换行。
    Closed { end: usize },
    /// 未闭合；围栏时 `end == len`，行内时仅为开界定符终点。
    Unclosed { end: usize },
}

/// 行首 `` ``` `` / `~~~`（缩进 ≤ 3）。规则贴近 CommonMark。
pub(super) fn match_fenced_code(s: &str, line_start: usize) -> Option<CodeHit> {
    let line = same_line(s, line_start);
    if !indent_ok_cm(line) {
        return None;
    }
    let trimmed = line.trim_start();
    let marker = *trimmed.as_bytes().first()?;
    if marker != b'`' && marker != b'~' {
        return None;
    }
    let open_len = trimmed.bytes().take_while(|&b| b == marker).count();
    if open_len < 3 {
        return None;
    }
    if marker == b'`' && trimmed[open_len..].contains('`') {
        return None;
    }

    let mut pos = line_start + line.len();
    if pos < s.len() {
        pos += 1; // 开围栏行后的 \n
    }

    while pos < s.len() {
        let line_end = s[pos..].find('\n').map(|n| pos + n).unwrap_or(s.len());
        let cur = &s[pos..line_end];
        let t = cur.trim_start();
        if indent_ok_cm(cur) {
            let close_len = t.bytes().take_while(|&b| b == marker).count();
            if close_len >= open_len && t[close_len..].trim().is_empty() {
                return Some(CodeHit::Closed {
                    end: include_trailing_nl(s, line_end),
                });
            }
        }
        if line_end >= s.len() {
            break;
        }
        pos = line_end + 1;
    }
    Some(CodeHit::Unclosed { end: s.len() })
}

/// 行内反引号：同长度闭合，不跨行；字节扫描，零分配。
/// 行中 ≥3 个 `` ` `` 也走这里（行首围栏由调用方先拦截）。
pub(super) fn match_inline_code(s: &str, start: usize) -> Option<CodeHit> {
    let bytes = s.as_bytes();
    if bytes.get(start) != Some(&b'`') {
        return None;
    }
    let open_len = bytes[start..].iter().take_while(|&&b| b == b'`').count();
    let after = start + open_len;
    let line_end = s[after..].find('\n').map(|n| after + n).unwrap_or(s.len());

    let mut pos = after;
    while pos < line_end {
        if bytes[pos] != b'`' {
            pos += 1;
            continue;
        }
        let run = bytes[pos..line_end]
            .iter()
            .take_while(|&&b| b == b'`')
            .count();
        if run == open_len {
            return Some(CodeHit::Closed { end: pos + run });
        }
        pos += run;
    }
    Some(CodeHit::Unclosed { end: after })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fence_closed() {
        let s = "```rust\nlet x = 1;\n```\n后文\n";
        assert_eq!(
            match_fenced_code(s, 0),
            Some(CodeHit::Closed {
                end: "```rust\nlet x = 1;\n```\n".len()
            })
        );
    }

    #[test]
    fn fence_unclosed() {
        let s = "```\n还在里面\n";
        assert_eq!(
            match_fenced_code(s, 0),
            Some(CodeHit::Unclosed { end: s.len() })
        );
    }

    #[test]
    fn fence_requires_line_start_indent() {
        // 缩进 4 空格：不算围栏开行
        assert_eq!(match_fenced_code("    ```\n", 0), None);
    }

    #[test]
    fn inline_closed_single() {
        let s = "见 `code` 处";
        let start = s.find('`').unwrap();
        assert_eq!(
            match_inline_code(s, start),
            Some(CodeHit::Closed {
                end: start + "`code`".len()
            })
        );
    }

    #[test]
    fn inline_unclosed() {
        let s = "坏掉的 ` 反引号";
        let start = s.find('`').unwrap();
        assert_eq!(
            match_inline_code(s, start),
            Some(CodeHit::Unclosed { end: start + 1 })
        );
    }

    // 行中 ```…``` 须整段匹配，不能因 open_len>=3 放弃
    #[test]
    fn inline_mid_line_triple() {
        let s = "foo ```bar``` baz";
        let start = s.find('`').unwrap();
        assert_eq!(
            match_inline_code(s, start),
            Some(CodeHit::Closed {
                end: start + "```bar```".len()
            })
        );
    }

    #[test]
    fn tilde_fence_not_closed_by_backtick() {
        let s = "~~~\ncode\n```\n";
        assert_eq!(
            match_fenced_code(s, 0),
            Some(CodeHit::Unclosed { end: s.len() })
        );
    }
}
