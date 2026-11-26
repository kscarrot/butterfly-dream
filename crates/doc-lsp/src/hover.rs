//! 实体「只露 name」后，hover 展示源码原文。

use doc_core::{
    line_starts, offset_in_span, position_to_offset, span_to_range, Document, EntityKind,
};
use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position, Range};

pub fn hover(doc: &Document, position: Position) -> Option<Hover> {
    let starts = line_starts(&doc.source);
    let offset = position_to_offset(&doc.source, &starts, position.line, position.character)?;

    for e in &doc.entities {
        if !offset_in_span(offset, e.span) {
            continue;
        }
        let raw = doc.source.get(e.span.start..e.span.end)?;
        let kind = kind_label(e.kind);
        let mut md = format!("```rymd\n{raw}\n```\n\n**{kind}** · `{}`", e.name);
        if let Some(id) = e.entity_id.as_deref() {
            md.push_str(&format!(" · id `{id}`"));
        }
        let (sl, sc, el, ec) = span_to_range(&doc.source, e.span);
        return Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: md,
            }),
            range: Some(Range::new(Position::new(sl, sc), Position::new(el, ec))),
        });
    }

    None
}

fn kind_label(k: EntityKind) -> &'static str {
    match k {
        EntityKind::Person => "人名",
        EntityKind::Place => "地名",
        EntityKind::Time => "时间",
        EntityKind::Office => "官职",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use doc_core::parse_editor;

    #[test]
    fn hover_shows_raw_entity() {
        let doc = parse_editor("@[买|liu-mai]生\n");
        let h = hover(&doc, Position::new(0, 2)).expect("hover");
        match h.contents {
            HoverContents::Markup(m) => {
                assert!(m.value.contains("@[买|liu-mai]"));
                assert!(m.value.contains("人名"));
                assert!(m.value.contains("liu-mai"));
            }
            _ => panic!("unexpected hover shape"),
        }
    }
}
