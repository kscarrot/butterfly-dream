//! 实体「只露 name」：向客户端推送 hide / name 区间。

use doc_core::{span_to_range, Document, EntityKind};
use serde::{Deserialize, Serialize};
use tower_lsp::lsp_types::{Position, Range, Url};

pub const METHOD: &str = "jmd/entityPresentation";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityPresentationParams {
    pub uri: Url,
    pub entities: Vec<EntityPresentation>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityPresentation {
    pub kind: String,
    pub name_range: Range,
    pub hide_ranges: Vec<Range>,
}

pub fn build(uri: &Url, doc: &Document) -> EntityPresentationParams {
    let source = &doc.source;
    let entities = doc
        .entities
        .iter()
        .map(|e| {
            let name_range = to_range(source, e.name_span);
            let mut hide_ranges = Vec::new();
            if e.span.start < e.name_span.start {
                hide_ranges.push(to_range(
                    source,
                    doc_core::Span::new(e.span.start, e.name_span.start),
                ));
            }
            if e.name_span.end < e.span.end {
                hide_ranges.push(to_range(
                    source,
                    doc_core::Span::new(e.name_span.end, e.span.end),
                ));
            }
            EntityPresentation {
                kind: kind_str(e.kind).into(),
                name_range,
                hide_ranges,
            }
        })
        .collect();
    EntityPresentationParams {
        uri: uri.clone(),
        entities,
    }
}

fn kind_str(k: EntityKind) -> &'static str {
    k.as_str()
}

fn to_range(source: &str, span: doc_core::Span) -> Range {
    let (sl, sc, el, ec) = span_to_range(source, span);
    Range::new(Position::new(sl, sc), Position::new(el, ec))
}

#[cfg(test)]
mod tests {
    use super::*;
    use doc_core::parse_editor;

    #[test]
    fn guangwu_line_hides_markers_keeps_name() {
        let src = "@[发|liu-fa]生$[舂陵节侯]@[买|liu-mai]\n";
        let doc = parse_editor(src);
        let uri = Url::parse("file:///t.rymd").unwrap();
        let p = build(&uri, &doc);
        assert_eq!(p.entities.len(), 3);

        let fa = &p.entities[0];
        assert_eq!(fa.kind, "person");
        assert_eq!(fa.hide_ranges.len(), 2);
        // name "发" starts after "@["
        assert_eq!(fa.name_range.start.character, 2);
        assert_eq!(fa.name_range.end.character, 3);

        let office = &p.entities[1];
        assert_eq!(office.kind, "office");
        assert_eq!(office.hide_ranges.len(), 2);
    }
}
