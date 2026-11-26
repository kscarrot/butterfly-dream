mod forbid_code;
mod frontmatter;
mod scan;
mod soft_join;
mod style_stack;
mod util;

use crate::ast::Document;

use frontmatter::extract_toml;
use scan::scan_extensions;
use soft_join::soft_join;

/// 解析模式：render 做 soft_join；编辑器 / LSP 跳过。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseMode {
    /// frontmatter → soft_join → scan（供 HTML 渲染）
    Render,
    /// frontmatter → 直接 scan 源 body（供 LSP；Span 对齐编辑器）
    Editor,
}

/// 解析源文本为 [`Document`]（默认 render 管线）。
pub fn parse_document(source: &str) -> Document {
    parse_document_with(source, ParseMode::Render)
}

/// 按模式解析。
pub fn parse_document_with(source: &str, mode: ParseMode) -> Document {
    let mut doc = Document {
        source: source.to_string(),
        ..Document::default()
    };

    let fm = extract_toml(source);
    doc.frontmatter = fm.value;
    doc.body_byte_offset = fm.body_byte_offset;
    if let Some(err) = fm.error {
        doc.parse_errors.push(err);
    }

    let (logical, logic_to_body) = match mode {
        ParseMode::Render => {
            let soft = soft_join(&fm.body);
            (soft.logical, soft.logic_to_body)
        }
        ParseMode::Editor => {
            let body = fm.body;
            let map: Vec<usize> = (0..body.len()).collect();
            (body.to_string(), map)
        }
    };

    doc.logical_body = logical;
    doc.logic_to_body = logic_to_body;

    let scanned = scan_extensions(
        &doc.logical_body,
        &doc.logic_to_body,
        doc.body_byte_offset,
    );
    doc.render_body = scanned.render_body;
    doc.entities = scanned.entities;
    doc.anchors = scanned.anchors;
    doc.styled_spans = scanned.styled_spans;
    doc.revision_refs = scanned.revision_refs;
    doc.revision_defs = scanned.revision_defs;
    doc.footnotes = scanned.footnotes;
    doc.parse_errors.extend(scanned.errors);

    doc
}
