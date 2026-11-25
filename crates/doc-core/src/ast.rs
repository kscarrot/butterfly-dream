use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::analyzer::Diagnostic;

/// 源文本中的字节区间（UTF-8，相对整份源文档）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

/// 历史实体类型：`@` 人名、`&` 地名、`#` 时间、`$` 官职。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EntityKind {
    Person,
    Place,
    Time,
    Office,
}

impl EntityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Person => "person",
            Self::Place => "place",
            Self::Time => "time",
            Self::Office => "office",
        }
    }

    pub fn from_marker(c: char) -> Option<Self> {
        match c {
            '@' => Some(Self::Person),
            '&' => Some(Self::Place),
            '#' => Some(Self::Time),
            '$' => Some(Self::Office),
            _ => None,
        }
    }
}

/// 括号形实体，如 `@[唐太宗|1024]`（左侧为原文可见字）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entity {
    pub kind: EntityKind,
    pub name: String,
    pub entity_id: Option<String>,
    pub span: Span,
}

/// 结构锚点：`{#id}`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Anchor {
    pub id: String,
    pub span: Span,
}

/// 染色：`[text]{.class}` 或 `==text=={color=#ff5500}`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StyledSpan {
    pub text: String,
    pub class: Option<String>,
    pub color: Option<String>,
    pub span: Span,
}

/// 侧栏修订引用（语法预留）：`[^c:rev_01]`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevisionRef {
    pub id: String,
    pub span: Span,
}

/// 修订定义（语法预留）：`[^c:rev_01]: …`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevisionDef {
    pub id: String,
    pub content: String,
    pub span: Span,
}

/// GFM 脚注的引用或定义。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FootnoteKind {
    Ref,
    Def,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Footnote {
    pub label: String,
    pub kind: FootnoteKind,
    pub span: Span,
}

/// 解析后的 jmd 文档。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub source: String,
    /// 解析后的 TOML frontmatter（JSON 对象；无则空对象）。
    pub frontmatter: Value,
    pub body_byte_offset: usize,
    /// 软分段合并后的正文（占位符在后续渲染阶段再替换）。
    #[serde(skip)]
    pub logical_body: String,
    /// `logical_body` 每个字节 → 原始 body 内偏移（再加 `body_byte_offset` 得绝对源偏移）。
    #[serde(skip)]
    pub logic_to_body: Vec<usize>,
    /// 已替换扩展占位符、供 markdown→HTML 的正文。
    #[serde(skip)]
    pub render_body: String,
    pub entities: Vec<Entity>,
    pub anchors: Vec<Anchor>,
    pub styled_spans: Vec<StyledSpan>,
    pub revision_refs: Vec<RevisionRef>,
    pub revision_defs: Vec<RevisionDef>,
    pub footnotes: Vec<Footnote>,
    pub parse_errors: Vec<Diagnostic>,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            source: String::new(),
            frontmatter: Value::Object(Default::default()),
            body_byte_offset: 0,
            logical_body: String::new(),
            logic_to_body: Vec::new(),
            render_body: String::new(),
            entities: Vec::new(),
            anchors: Vec::new(),
            styled_spans: Vec::new(),
            revision_refs: Vec::new(),
            revision_defs: Vec::new(),
            footnotes: Vec::new(),
            parse_errors: Vec::new(),
        }
    }
}
