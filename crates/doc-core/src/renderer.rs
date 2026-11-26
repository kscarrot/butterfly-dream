use markdown::{
    CompileOptions, Constructs, Options, ParseOptions,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ast::{Document, EntityKind};
use crate::placeholder::Placeholder;

/// 预览用结构化输出（正文 HTML + 侧栏数据）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderOutput {
    pub frontmatter: Value,
    pub body_html: String,
    pub entities: Vec<EntityView>,
    pub revisions: Vec<RevisionView>,
}

/// 实体侧栏 / 索引条目。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityView {
    pub kind: String,
    pub name: String,
    pub entity_id: Option<String>,
}

/// 修订侧栏卡片（语义后续再补，本版本可为空）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevisionView {
    pub id: String,
    pub content: Option<String>,
    pub line: Option<u32>,
}

fn jmd_options() -> Options {
    let mut parse = ParseOptions::gfm();
    parse.constructs = Constructs {
        frontmatter: false,
        math_flow: false,
        math_text: false,
        ..Constructs::gfm()
    };
    Options {
        parse,
        compile: CompileOptions::gfm(),
    }
}

/// HTML 属性 / 文本转义（example 等也复用）。
pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn entity_html(kind: EntityKind, name: &str, id: Option<&str>) -> String {
    let kind_s = kind.as_str();
    let class = format!("jmd-entity jmd-{kind_s}");
    match id {
        Some(eid) => format!(
            r#"<a class="{class}" data-kind="{kind_s}" data-id="{}" href="entity://{kind_s}/{}">{}</a>"#,
            html_escape(eid),
            html_escape(eid),
            html_escape(name)
        ),
        None => format!(
            r#"<span class="{class}" data-kind="{kind_s}">{}</span>"#,
            html_escape(name)
        ),
    }
}

/// 转义普通文本，保留已嵌入的 `%%JMD_*%%` 占位符（供染色内层实体等二次替换）。
fn escape_preserving_placeholders(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        if s[i..].starts_with("%%JMD_") {
            if let Some(rel) = s[i + 6..].find("%%") {
                let end = i + 6 + rel + 2;
                out.push_str(&s[i..end]);
                i = end;
                continue;
            }
        }
        let ch = s[i..].chars().next().unwrap();
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
        i += ch.len_utf8();
    }
    out
}

fn style_html(text: &str, class: &str) -> String {
    format!(
        r#"<span class="jmd-style {}">{}</span>"#,
        html_escape(class),
        escape_preserving_placeholders(text)
    )
}

fn revision_html(id: &str) -> String {
    let id = html_escape(id);
    format!(
        "<sup class=\"jmd-revision\" data-rev=\"{id}\"><a href=\"#jmd-rev-{id}\">c:{id}</a></sup>"
    )
}

/// 将文档渲染为结构化预览（JSON 字段 + HTML）。
pub fn render(doc: &Document) -> RenderOutput {
    let mut md_src = doc.render_body.clone();

    // 先把占位符换成内联 HTML；需打开 allow_dangerous_html 才能原样通过
    let mut opts = jmd_options();
    opts.compile.allow_dangerous_html = true;

    // 先展开染色：外层 index 更大且内文可含内层 Style/叶子占位符，故自高向低替换
    for (i, s) in doc.styled_spans.iter().enumerate().rev() {
        let html = style_html(&s.text, &s.class);
        md_src = md_src.replace(&Placeholder::Style.format(i), &html);
    }
    for (i, e) in doc.entities.iter().enumerate() {
        let html = entity_html(e.kind, &e.name, e.entity_id.as_deref());
        md_src = md_src.replace(&Placeholder::Entity.format(i), &html);
    }
    for (i, r) in doc.revision_refs.iter().enumerate() {
        let html = revision_html(&r.id);
        md_src = md_src.replace(&Placeholder::Revision.format(i), &html);
    }
    for (i, a) in doc.anchors.iter().enumerate() {
        let html = format!(r#"<span id="{}"></span>"#, html_escape(&a.id));
        md_src = md_src.replace(&Placeholder::Anchor.format(i), &html);
    }

    let body_html = match markdown::to_html_with_options(&md_src, &opts) {
        Ok(html) => html,
        Err(e) => format!("<!-- markdown error: {e} --><pre>{}</pre>", html_escape(&md_src)),
    };

    let entities = doc
        .entities
        .iter()
        .map(|e| EntityView {
            kind: e.kind.as_str().to_string(),
            name: e.name.clone(),
            entity_id: e.entity_id.clone(),
        })
        .collect();

    RenderOutput {
        frontmatter: doc.frontmatter.clone(),
        body_html,
        entities,
        revisions: Vec::new(),
    }
}
