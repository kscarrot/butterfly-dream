use serde_json::Value;

use crate::analyzer::Diagnostic;
use crate::ast::Span;

pub struct FrontmatterResult {
    pub value: Value,
    /// 正文在整份源文档中的起始字节偏移。
    pub body_byte_offset: usize,
    pub body: String,
    pub error: Option<Diagnostic>,
}

/// 剥离开头的 `+++ ... +++` TOML frontmatter 并解析。
pub fn parse_frontmatter(source: &str) -> FrontmatterResult {
    let bom = source.starts_with('\u{feff}');
    let start = if bom { '\u{feff}'.len_utf8() } else { 0 };
    let text = &source[start..];

    if !text.starts_with("+++") {
        return FrontmatterResult {
            value: Value::Object(Default::default()),
            body_byte_offset: start,
            body: text.to_string(),
            error: None,
        };
    }

    // 开围栏须为文件首行：+++\n
    let after_open = match text.strip_prefix("+++") {
        Some(rest) => {
            if let Some(r) = rest.strip_prefix('\n') {
                r
            } else if let Some(r) = rest.strip_prefix("\r\n") {
                r
            } else {
                return FrontmatterResult {
                    value: Value::Object(Default::default()),
                    body_byte_offset: start,
                    body: text.to_string(),
                    error: None,
                };
            }
        }
        None => {
            return FrontmatterResult {
                value: Value::Object(Default::default()),
                body_byte_offset: start,
                body: text.to_string(),
                error: None,
            };
        }
    };

    let close_pat = "\n+++";
    let Some(rel_end) = after_open.find(close_pat) else {
        let err = Diagnostic::error(
            "Frontmatter 缺少闭合 +++",
            Some(Span::new(start, source.len())),
        );
        return FrontmatterResult {
            value: Value::Object(Default::default()),
            body_byte_offset: start,
            body: text.to_string(),
            error: Some(err),
        };
    };

    let toml_text = &after_open[..rel_end];
    let after_close = &after_open[rel_end + close_pat.len()..];
    let body = after_close
        .strip_prefix('\n')
        .or_else(|| after_close.strip_prefix("\r\n"))
        .unwrap_or(after_close);

    let fm_start = start;
    let body_byte_offset = source.len() - body.len();

    let (value, error) = match toml_text.parse::<toml::Table>() {
        Ok(table) => match serde_json::to_value(table) {
            Ok(v) => (v, None),
            Err(e) => (
                Value::Object(Default::default()),
                Some(Diagnostic::error(
                    format!("Frontmatter TOML 无效: {e}"),
                    Some(Span::new(fm_start, body_byte_offset)),
                )),
            ),
        },
        Err(e) => (
            Value::Object(Default::default()),
            Some(Diagnostic::error(
                format!("Frontmatter TOML 无效: {e}"),
                Some(Span::new(fm_start, body_byte_offset)),
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
