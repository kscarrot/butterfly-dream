//! 将 frontmatter / 脚注定义 / 修订定义映射为 LSP FoldingRange。

use doc_core::{span_to_range, Document, FootnoteKind};
use tower_lsp::lsp_types::{FoldingRange, FoldingRangeKind};

pub fn folding_ranges(doc: &Document) -> Vec<FoldingRange> {
    let mut out = Vec::new();
    let source = &doc.source;

    if doc.body_byte_offset > 0 {
        // frontmatter：源开头到 body 起点（通常含闭合 +++ 行）
        let end = doc.body_byte_offset.saturating_sub(1);
        let (sl, _, el, _) = span_to_range(source, doc_core::Span::new(0, end.max(1)));
        if el > sl {
            out.push(FoldingRange {
                start_line: sl,
                start_character: None,
                end_line: el,
                end_character: None,
                kind: Some(FoldingRangeKind::Region),
                collapsed_text: Some("frontmatter".into()),
            });
        }
    }

    for f in &doc.footnotes {
        if f.kind != FootnoteKind::Def {
            continue;
        }
        push_span_fold(&mut out, source, f.span, "footnote");
    }
    for r in &doc.revision_defs {
        push_span_fold(&mut out, source, r.span, "revision");
    }

    out
}

fn push_span_fold(out: &mut Vec<FoldingRange>, source: &str, span: doc_core::Span, label: &str) {
    let (sl, _, el, _) = span_to_range(source, span);
    if el >= sl {
        out.push(FoldingRange {
            start_line: sl,
            start_character: None,
            end_line: el,
            end_character: None,
            kind: Some(FoldingRangeKind::Region),
            collapsed_text: Some(label.into()),
        });
    }
}
