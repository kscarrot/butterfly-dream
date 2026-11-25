use serde::{Deserialize, Serialize};

use crate::ast::{Document, FootnoteKind, Span};

/// 诊断级别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// 闭环校验得到的一条诊断。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Option<Span>,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            span,
        }
    }

    pub fn warning(message: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            span,
        }
    }
}

/// 跑闭环检查（重复锚点、脚注成对等）。修订 `[^c:…]` 本阶段不校验。
pub fn validate(doc: &Document) -> Vec<Diagnostic> {
    let mut out = doc.parse_errors.clone();

    // 结构锚点重名
    let mut seen: Vec<(&str, Span)> = Vec::new();
    for a in &doc.anchors {
        if let Some((_, prev)) = seen.iter().find(|(id, _)| *id == a.id.as_str()) {
            out.push(Diagnostic::warning(
                format!("ID \"{}\" 重复定义", a.id),
                Some(a.span),
            ));
            let _ = prev;
        } else {
            seen.push((a.id.as_str(), a.span));
        }
    }

    // 脚注引用/定义闭环（修订 label 不走这里）
    let refs: Vec<_> = doc
        .footnotes
        .iter()
        .filter(|f| f.kind == FootnoteKind::Ref)
        .collect();
    let defs: Vec<_> = doc
        .footnotes
        .iter()
        .filter(|f| f.kind == FootnoteKind::Def)
        .collect();

    for r in &refs {
        if !defs.iter().any(|d| d.label == r.label) {
            out.push(Diagnostic::error(
                format!("缺失脚注定义 [^{}]", r.label),
                Some(r.span),
            ));
        }
    }
    for d in &defs {
        if !refs.iter().any(|r| r.label == d.label) {
            out.push(Diagnostic::warning(
                format!("未使用的脚注 [^{}]", d.label),
                Some(d.span),
            ));
        }
    }

    out
}
