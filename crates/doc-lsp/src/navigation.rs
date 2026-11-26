//! 脚注 / 修订的 Go to Definition。

use doc_core::{
    line_starts, offset_in_span, offset_to_position, position_to_offset, span_to_range, Document,
    FootnoteKind,
};
use tower_lsp::lsp_types::{Location, Position, Range, Url};

pub fn goto_definition(doc: &Document, uri: &Url, position: Position) -> Option<Location> {
    let starts = line_starts(&doc.source);
    let offset = position_to_offset(&doc.source, &starts, position.line, position.character)?;

    for f in &doc.footnotes {
        if !offset_in_span(offset, f.span) {
            continue;
        }
        let target = match f.kind {
            FootnoteKind::Ref => doc
                .footnotes
                .iter()
                .find(|x| x.kind == FootnoteKind::Def && x.label == f.label),
            FootnoteKind::Def => doc
                .footnotes
                .iter()
                .find(|x| x.kind == FootnoteKind::Ref && x.label == f.label),
        };
        if let Some(t) = target {
            return Some(location(uri, &doc.source, t.span));
        }
    }

    for r in &doc.revision_refs {
        if offset_in_span(offset, r.span) {
            if let Some(def) = doc.revision_defs.iter().find(|d| d.id == r.id) {
                return Some(location(uri, &doc.source, def.span));
            }
            return None;
        }
    }

    for d in &doc.revision_defs {
        if offset_in_span(offset, d.span) {
            if let Some(r) = doc.revision_refs.iter().find(|x| x.id == d.id) {
                return Some(location(uri, &doc.source, r.span));
            }
        }
    }

    None
}

fn location(uri: &Url, source: &str, span: doc_core::Span) -> Location {
    let (sl, sc, el, ec) = span_to_range(source, span);
    Location {
        uri: uri.clone(),
        range: Range::new(Position::new(sl, sc), Position::new(el, ec)),
    }
}

#[allow(dead_code)]
pub fn position_at(source: &str, offset: usize) -> Position {
    let starts = line_starts(source);
    let (l, c) = offset_to_position(source, &starts, offset);
    Position::new(l, c)
}
