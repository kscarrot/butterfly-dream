mod frontmatter;
mod scan;
mod soft_join;

use crate::ast::Document;

use frontmatter::parse_frontmatter;
use scan::scan_extensions;
use soft_join::soft_join;

/// 解析源文本为 [`Document`]（SPEC 管线 ①–③：frontmatter → 软分段 → 扩展扫描）。
pub fn parse_document(source: &str) -> Document {
    let mut doc = Document {
        source: source.to_string(),
        ..Document::default()
    };

    let fm = parse_frontmatter(source);
    doc.frontmatter = fm.value;
    doc.body_byte_offset = fm.body_byte_offset;
    if let Some(err) = fm.error {
        doc.parse_errors.push(err);
    }

    let soft = soft_join(&fm.body);
    doc.logical_body = soft.logical;
    doc.logic_to_body = soft.logic_to_body;

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
