//! jmd 文档核心库：解析、校验与渲染。
//!
//! 本 crate 不做文件 I/O，便于同时编译为原生 LSP 与 WASM。

pub mod analyzer;
pub mod ast;
pub mod parser;
mod placeholder;
pub mod position;
pub mod renderer;

pub use analyzer::{validate, Diagnostic, Severity};
pub use ast::{
    Anchor, Document, Entity, EntityKind, Footnote, FootnoteKind, RevisionDef, RevisionRef, Span,
    StyledSpan,
};
pub use parser::ParseMode;
pub use position::{
    line_starts, offset_in_span, offset_to_position, position_to_offset, span_to_range,
};
pub use renderer::{html_escape, render, EntityView, RenderOutput, RevisionView};

/// 将源文本解析为 [`Document`]（含 soft_join，供渲染）。
pub fn parse(source: &str) -> Document {
    parser::parse_document_with(source, ParseMode::Render)
}

/// 编辑器 / LSP 解析：跳过 soft_join，Span 对齐源码行。
pub fn parse_editor(source: &str) -> Document {
    parser::parse_document_with(source, ParseMode::Editor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_smoke() {
        let doc = parse("hello");
        assert_eq!(doc.source, "hello");
        assert!(doc.frontmatter.as_object().unwrap().is_empty());
    }

    #[test]
    fn soft_join_and_entity() {
        let src = "+++\ntitle = \"t\"\n+++\n\n@[光武|liu]甲，\\\n\n乙。\n";
        let doc = parse(src);
        assert_eq!(doc.frontmatter["title"], "t");
        assert_eq!(doc.entities.len(), 1);
        assert_eq!(doc.entities[0].name, "光武");
        let out = render(&doc);
        assert!(out.body_html.contains("光武"));
        assert!(out.body_html.contains("甲，乙。") || out.body_html.contains("甲，乙"));
        assert!(!out.body_html.contains('\\'));
    }

    #[test]
    fn style_nests_and_wraps_entity() {
        let doc = parse("[&[南阳|ny]]{.note}\n[[内]{.em}外]{.note}\n");
        assert_eq!(doc.entities.len(), 1);
        assert_eq!(doc.styled_spans.len(), 3);
        let out = render(&doc);
        assert!(out.body_html.contains(r#"class="jmd-style note""#));
        assert!(out.body_html.contains(r#"class="jmd-style em""#));
        assert!(out.body_html.contains("南阳"));
        assert!(out.body_html.contains("data-id=\"ny\""));
        assert!(!out.body_html.contains("%%JMD_"));
    }

    #[test]
    fn parse_editor_skips_soft_join_and_sets_name_span() {
        let src = "@[光武|liu]甲，\\\n\n乙。\n";
        let ed = parse_editor(src);
        assert_eq!(ed.entities.len(), 1);
        assert_eq!(ed.entities[0].name, "光武");
        assert_eq!(&src[ed.entities[0].name_span.start..ed.entities[0].name_span.end], "光武");
        // 编辑器模式不合并软断：logical 仍含反斜杠
        assert!(ed.logical_body.contains('\\'));

        let rendered = parse(src);
        assert!(!rendered.logical_body.contains('\\'));
    }
}
