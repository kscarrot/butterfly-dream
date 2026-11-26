use doc_core::{parse, render, validate, EntityKind, FootnoteKind, Severity};

fn read_example(name: &str) -> String {
    let path = format!("{}/examples/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

#[test]
fn guangwu_conformance() {
    let src = read_example("guangwu.rymd");
    let doc = parse(&src);
    let diags = validate(&doc);
    let out = render(&doc);

    assert_eq!(doc.frontmatter["title"], "后汉书 · 光武帝纪（摘录）");
    assert_eq!(doc.frontmatter["author"], "范晔");

    assert!(
        doc.entities.iter().any(|e| {
            e.kind == EntityKind::Person
                && e.name == "发"
                && e.entity_id.as_deref() == Some("liu-fa")
        }),
        "expected surface form @[发|liu-fa], got {:?}",
        doc.entities
    );
    assert!(
        doc.entities.iter().any(|e| e.kind == EntityKind::Person && e.entity_id.as_deref() == Some("liu-xiu")),
        "expected person liu-xiu, got {:?}",
        doc.entities
    );
    assert!(doc.entities.iter().any(|e| e.kind == EntityKind::Place));
    assert!(doc.entities.iter().any(|e| e.kind == EntityKind::Time));
    assert!(doc.entities.iter().any(|e| e.kind == EntityKind::Office));

    assert!(doc.anchors.iter().any(|a| a.id == "guangwu-1"));
    assert!(doc.revision_refs.iter().any(|r| r.id == "rev_01"));
    assert!(doc.footnotes.iter().any(|f| f.label == "1" && f.kind == FootnoteKind::Ref));
    assert!(doc.footnotes.iter().any(|f| f.label == "1" && f.kind == FootnoteKind::Def));

    assert!(
        !diags.iter().any(|d| d.severity == Severity::Error),
        "unexpected errors: {diags:?}"
    );

    // 软分段后正文应连成一段
    assert!(
        out.body_html.contains("九世之孙") && out.body_html.contains("蔡阳"),
        "joined text missing: {}",
        out.body_html
    );
    assert!(
        !out.body_html.contains("蔡阳</span>人， <"),
        "soft-join left ASCII gap before next entity: {}",
        out.body_html
    );
    // GFM 脚注须带可点击锚点
    assert!(
        out.body_html.contains("href=") && (out.body_html.contains("footnote") || out.body_html.contains("fn")),
        "expected footnote links in {}",
        out.body_html
    );
    assert!(out.body_html.contains("data-kind=\"person\"") || out.body_html.contains("jmd-person"));
}

#[test]
fn broken_links_conformance() {
    let src = read_example("broken-links.rymd");
    let doc = parse(&src);
    let diags = validate(&doc);

    assert!(
        diags.iter().any(|d| d.severity == Severity::Warning && d.message.contains("dup-id")),
        "expected duplicate id warning: {diags:?}"
    );
    assert!(
        diags.iter().any(|d| d.severity == Severity::Error && d.message.contains("未闭合")),
        "expected unclosed entity error: {diags:?}"
    );
    assert!(
        diags.iter().any(|d| d.severity == Severity::Error && d.message.contains("[^99]")),
        "expected missing footnote error: {diags:?}"
    );
    assert!(
        !diags.iter().any(|d| d.message.contains("orphan") || d.message.contains("rev")),
        "revision should not error: {diags:?}"
    );
    assert!(doc.revision_refs.iter().any(|r| r.id == "orphan"));
}
