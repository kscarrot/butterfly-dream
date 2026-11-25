//! jmd 文档核心库：解析、校验与渲染。
//!
//! 本 crate 不做文件 I/O，便于同时编译为原生 LSP 与 WASM。

pub mod analyzer;
pub mod ast;
pub mod parser;
pub mod renderer;

pub use analyzer::{validate, Diagnostic, Severity};
pub use ast::{
    Anchor, Document, Entity, EntityKind, Footnote, FootnoteKind, RevisionDef, RevisionRef, Span,
    StyledSpan,
};
pub use renderer::{render, EntityView, RenderOutput, RevisionView};

/// 将源文本解析为 [`Document`]。
pub fn parse(source: &str) -> Document {
    parser::parse_document(source)
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
}
