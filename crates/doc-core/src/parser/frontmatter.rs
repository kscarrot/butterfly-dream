use serde_json::Value;

use super::util::{strip_eol, strip_eol_or_eof};
use crate::analyzer::Diagnostic;
use crate::ast::Span;

pub struct FrontmatterResult {
    pub value: Value,
    /// 正文在整份源文档中的起始字节偏移。
    pub body_byte_offset: usize,
    pub body: String,
    pub error: Option<Diagnostic>,
}

/// 提取开头的 `+++ ... +++` TOML frontmatter 并解析。
pub fn extract_toml(source: &str) -> FrontmatterResult {
    let text = source.strip_prefix('\u{feff}').unwrap_or(source);
    let start = source.len() - text.len();

    let fallback = || FrontmatterResult {
        value: Value::Object(Default::default()),
        body_byte_offset: start,
        body: text.to_string(),
        error: None,
    };

    // 开围栏须独占首行：+++\n 或 +++\r\n
    let Some(after_open) = text.strip_prefix("+++").and_then(strip_eol) else {
        return fallback();
    };

    let Some((toml_text, body)) = split_closing_fence(after_open) else {
        return FrontmatterResult {
            value: Value::Object(Default::default()),
            body_byte_offset: start,
            body: text.to_string(),
            error: Some(Diagnostic::error(
                "Frontmatter 缺少闭合 +++",
                Some(Span::new(start, source.len())),
            )),
        };
    };

    let body_byte_offset = source.len() - body.len();

    let parsed = toml_text
        .parse::<toml::Table>()
        .map_err(|e| e.to_string())
        .and_then(|table| serde_json::to_value(table).map_err(|e| e.to_string()));

    let (value, error) = match parsed {
        Ok(v) => (v, None),
        Err(msg) => (
            Value::Object(Default::default()),
            Some(Diagnostic::error(
                format!("Frontmatter TOML 无效: {msg}"),
                Some(Span::new(start, body_byte_offset)),
            )),
        ),
    };

    FrontmatterResult {
        value,
        body_byte_offset,
        body: body.to_string(),
        error,
    }
}

/// 在开围栏之后寻找合法闭围栏 `+++`，返回 `(toml_text, body)`。
fn split_closing_fence(after_open: &str) -> Option<(&str, &str)> {
    // 空 frontmatter：开围栏后紧接闭围栏（+++\n+++）
    if let Some(rest) = after_open.strip_prefix("+++") {
        if let Some(body) = strip_eol_or_eof(rest) {
            return Some(("", body));
        }
    }

    let mut search_from = 0;
    while let Some(rel) = after_open[search_from..].find("\n+++") {
        let newline_pos = search_from + rel;
        let after_fence = &after_open[newline_pos + "\n+++".len()..];

        if let Some(body) = strip_eol_or_eof(after_fence) {
            // 闭围栏前若为 \r\n，去掉 toml_text 末尾残留的 \r
            let toml_end = after_open[..newline_pos]
                .strip_suffix('\r')
                .map_or(newline_pos, |s| s.len());
            return Some((&after_open[..toml_end], body));
        }

        // `+++note` 一类伪闭围栏，继续向后找
        search_from = newline_pos + "\n+++".len();
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_frontmatter_is_ok() {
        let r = extract_toml("+++\n+++\nbody\n");
        assert!(r.error.is_none());
        assert!(r.value.as_object().unwrap().is_empty());
        assert_eq!(r.body, "body\n");
        assert_eq!(r.body_byte_offset, "+++\n+++\n".len());
    }

    #[test]
    fn closing_fence_must_be_alone_on_line() {
        // 正文里的 `+++note` 不得被当成闭围栏
        let src = "+++\ntitle = \"t\"\n+++\nbody with\n+++note\nstill\n";
        let r = extract_toml(src);
        assert!(r.error.is_none(), "{:?}", r.error);
        assert_eq!(r.value["title"], "t");
        assert_eq!(r.body, "body with\n+++note\nstill\n");
    }

    #[test]
    fn non_fence_line_inside_frontmatter_is_toml_error() {
        // frontmatter 内的 `+++note` 不是闭围栏，会落入 TOML 文本
        let src = "+++\ntitle = \"t\"\n+++note\n+++\nbody\n";
        let r = extract_toml(src);
        assert!(r.error.is_some());
        assert!(r.error.unwrap().message.contains("TOML"));
        assert_eq!(r.body, "body\n");
    }

    #[test]
    fn crlf_closing_fence() {
        let src = "+++\r\ntitle = \"t\"\r\n+++\r\n\r\nbody\n";
        let r = extract_toml(src);
        assert!(r.error.is_none(), "{:?}", r.error);
        assert_eq!(r.value["title"], "t");
        assert_eq!(r.body, "\r\nbody\n");
    }

    #[test]
    fn bom_is_skipped() {
        let src = "\u{feff}+++\ntitle = \"t\"\n+++\nbody\n";
        let r = extract_toml(src);
        assert!(r.error.is_none());
        assert_eq!(r.value["title"], "t");
        assert_eq!(r.body_byte_offset, "\u{feff}+++\ntitle = \"t\"\n+++\n".len());
    }

    #[test]
    fn missing_closing_fence_errors() {
        let r = extract_toml("+++\ntitle = \"t\"\n");
        assert!(r.error.is_some());
        assert_eq!(r.body, "+++\ntitle = \"t\"\n");
    }
}
