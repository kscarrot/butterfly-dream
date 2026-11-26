//! 实体 / 脚注 / 修订的 semantic tokens。

use doc_core::{line_starts, offset_to_position, Document, EntityKind};
use tower_lsp::lsp_types::{
    SemanticToken, SemanticTokenType, SemanticTokens, SemanticTokensLegend,
};

pub fn legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: TOKEN_TYPES.iter().copied().map(SemanticTokenType::from).collect(),
        token_modifiers: Vec::new(),
    }
}

const TOKEN_TYPES: &[&str] = &[
    "jmd_entity_person",
    "jmd_entity_place",
    "jmd_entity_time",
    "jmd_entity_office",
    "jmd_footnote",
    "jmd_revision",
];

fn entity_type_index(kind: EntityKind) -> u32 {
    match kind {
        EntityKind::Person => 0,
        EntityKind::Place => 1,
        EntityKind::Time => 2,
        EntityKind::Office => 3,
    }
}

pub fn semantic_tokens(doc: &Document) -> SemanticTokens {
    let source = &doc.source;
    let starts = line_starts(source);
    let mut raw: Vec<(u32, u32, u32, u32)> = Vec::new(); // line, col, len_utf16, type

    for e in &doc.entities {
        let (sl, sc) = offset_to_position(source, &starts, e.name_span.start);
        let (el, ec) = offset_to_position(source, &starts, e.name_span.end);
        if sl == el {
            raw.push((sl, sc, ec.saturating_sub(sc), entity_type_index(e.kind)));
        }
    }
    for f in &doc.footnotes {
        push_token(&mut raw, source, &starts, f.span, 4);
    }
    for r in &doc.revision_refs {
        push_token(&mut raw, source, &starts, r.span, 5);
    }
    for r in &doc.revision_defs {
        push_token(&mut raw, source, &starts, r.span, 5);
    }

    raw.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    let mut data = Vec::new();
    let mut prev_line = 0u32;
    let mut prev_col = 0u32;
    for (line, col, len, ty) in raw {
        let delta_line = line - prev_line;
        let delta_start = if delta_line == 0 {
            col.saturating_sub(prev_col)
        } else {
            col
        };
        data.push(SemanticToken {
            delta_line,
            delta_start,
            length: len,
            token_type: ty,
            token_modifiers_bitset: 0,
        });
        prev_line = line;
        prev_col = col;
    }

    SemanticTokens {
        result_id: None,
        data,
    }
}

fn push_token(
    raw: &mut Vec<(u32, u32, u32, u32)>,
    source: &str,
    starts: &[usize],
    span: doc_core::Span,
    ty: u32,
) {
    let (sl, sc) = offset_to_position(source, starts, span.start);
    let (el, ec) = offset_to_position(source, starts, span.end);
    if sl == el {
        raw.push((sl, sc, ec.saturating_sub(sc), ty));
    }
}
