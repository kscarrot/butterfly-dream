//! 扫描逻辑正文中的 JMD 扩展，并生成带占位符的渲染正文。

use crate::analyzer::Diagnostic;
use crate::ast::{
    Anchor, Entity, EntityKind, Footnote, FootnoteKind, RevisionDef, RevisionRef, Span, StyledSpan,
};

pub struct ScanResult {
    pub render_body: String,
    pub entities: Vec<Entity>,
    pub anchors: Vec<Anchor>,
    pub styled_spans: Vec<StyledSpan>,
    pub revision_refs: Vec<RevisionRef>,
    pub revision_defs: Vec<RevisionDef>,
    pub footnotes: Vec<Footnote>,
    pub errors: Vec<Diagnostic>,
}

struct Mapper<'a> {
    body_byte_offset: usize,
    logic_to_body: &'a [usize],
}

impl Mapper<'_> {
    fn span(&self, logic_start: usize, logic_end: usize) -> Span {
        let b0 = self
            .logic_to_body
            .get(logic_start)
            .copied()
            .unwrap_or(logic_start);
        let b1 = if logic_end == 0 {
            b0
        } else {
            self.logic_to_body
                .get(logic_end - 1)
                .copied()
                .unwrap_or(b0)
                + 1
        };
        Span::new(self.body_byte_offset + b0, self.body_byte_offset + b1)
    }
}

fn next_char_boundary(s: &str, i: usize) -> usize {
    if i >= s.len() {
        return s.len();
    }
    let mut j = i + 1;
    while j < s.len() && !s.is_char_boundary(j) {
        j += 1;
    }
    j
}

fn in_code_fence_or_inline(logical: &str, idx: usize) -> bool {
    if !logical.is_char_boundary(idx) {
        return true;
    }
    // 粗略判断：统计 idx 之前的围栏行 / 反引号奇偶
    let before = &logical[..idx];
    // 代码围栏：``` / ~~~ 出现奇数次则仍在围栏内
    let mut fence = false;
    for line in before.split('\n') {
        let t = line.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            fence = !fence;
        }
    }
    if fence {
        return true;
    }
    // 行内代码：反引号奇数个
    let ticks = before.matches('`').count();
    ticks % 2 == 1
}

/// 扫描 `logical`，产出 AST 节点，以及供 markdown 渲染的占位符正文。
pub fn scan_extensions(
    logical: &str,
    logic_to_body: &[usize],
    body_byte_offset: usize,
) -> ScanResult {
    let map = Mapper {
        body_byte_offset,
        logic_to_body,
    };

    let mut entities = Vec::new();
    let mut anchors = Vec::new();
    let mut styled_spans = Vec::new();
    let mut revision_refs = Vec::new();
    let mut revision_defs = Vec::new();
    let mut footnotes = Vec::new();
    let mut errors = Vec::new();

    // 替换区间：(start, end, replacement)
    let mut repl: Vec<(usize, usize, String)> = Vec::new();
    let mut i = 0;
    let mut prev_was_newline = true;

    while i < logical.len() {
        // 保证落在 UTF-8 字符边界上
        if !logical.is_char_boundary(i) {
            i += 1;
            continue;
        }

        if in_code_fence_or_inline(logical, i) {
            i = next_char_boundary(logical, i);
            prev_was_newline = logical.as_bytes().get(i.wrapping_sub(1)) == Some(&b'\n');
            continue;
        }

        // 行首的定义块（修订 / 脚注）
        if prev_was_newline {
            if let Some((end, id, content)) = match_revision_def(logical, i) {
                revision_defs.push(RevisionDef {
                    id,
                    content,
                    span: map.span(i, end),
                });
                let line_end = logical[i..]
                    .find('\n')
                    .map(|n| i + n)
                    .unwrap_or(logical.len());
                repl.push((i, line_end, String::new()));
                i = line_end;
                prev_was_newline = false;
                continue;
            }
            if let Some((end, label, _content)) = match_footnote_def(logical, i) {
                if !label.starts_with("c:") {
                    footnotes.push(Footnote {
                        label,
                        kind: FootnoteKind::Def,
                        span: map.span(i, end),
                    });
                }
                // GFM 脚注定义留给 markdown-rs，原文保留
                i = next_char_boundary(logical, i);
                prev_was_newline = false;
                continue;
            }
        }

        let ch = logical[i..].chars().next().unwrap_or('\0');

        // 括号实体 @[...]
        if let Some(kind) = EntityKind::from_marker(ch) {
            if logical[i + ch.len_utf8()..].starts_with('[') {
                match read_bracket_entity(logical, i) {
                    Ok((end, name, entity_id)) => {
                        let idx = entities.len();
                        entities.push(Entity {
                            kind,
                            name,
                            entity_id,
                            span: map.span(i, end),
                        });
                        repl.push((i, end, format!("%%JMD_E_{idx}%%")));
                        i = end;
                        prev_was_newline = false;
                        continue;
                    }
                    Err(end_hint) => {
                        errors.push(Diagnostic::error(
                            "未闭合的实体标注",
                            Some(map.span(i, end_hint)),
                        ));
                        i = next_char_boundary(logical, i);
                        prev_was_newline = false;
                        continue;
                    }
                }
            }
        }

        if logical[i..].starts_with("==") {
            if let Some((end, text, color)) = match_color_span(logical, i) {
                let idx = styled_spans.len();
                styled_spans.push(StyledSpan {
                    text,
                    class: None,
                    color: Some(color),
                    span: map.span(i, end),
                });
                repl.push((i, end, format!("%%JMD_S_{idx}%%")));
                i = end;
                prev_was_newline = false;
                continue;
            }
        }

        if ch == '[' {
            if let Some((end, text, class)) = match_class_span(logical, i) {
                let idx = styled_spans.len();
                styled_spans.push(StyledSpan {
                    text,
                    class: Some(class),
                    color: None,
                    span: map.span(i, end),
                });
                repl.push((i, end, format!("%%JMD_S_{idx}%%")));
                i = end;
                prev_was_newline = false;
                continue;
            }
        }

        if logical[i..].starts_with("[^c:") {
            if let Some((end, id)) = match_revision_ref(logical, i) {
                let idx = revision_refs.len();
                revision_refs.push(RevisionRef {
                    id,
                    span: map.span(i, end),
                });
                repl.push((i, end, format!("%%JMD_R_{idx}%%")));
                i = end;
                prev_was_newline = false;
                continue;
            }
        }

        if logical[i..].starts_with("[^") {
            if let Some((end, label)) = match_footnote_ref(logical, i) {
                if !label.starts_with("c:") {
                    footnotes.push(Footnote {
                        label,
                        kind: FootnoteKind::Ref,
                        span: map.span(i, end),
                    });
                    i = end;
                    prev_was_newline = false;
                    continue;
                }
            }
        }

        if logical[i..].starts_with("{#") {
            if let Some((end, id)) = match_anchor(logical, i) {
                anchors.push(Anchor {
                    id,
                    span: map.span(i, end),
                });
                repl.push((i, end, format!("%%JMD_A_{}%%", anchors.len() - 1)));
                i = end;
                prev_was_newline = false;
                continue;
            }
        }

        prev_was_newline = ch == '\n';
        i = next_char_boundary(logical, i);
    }

    let render_body = apply_replacements(logical, &repl);

    ScanResult {
        render_body,
        entities,
        anchors,
        styled_spans,
        revision_refs,
        revision_defs,
        footnotes,
        errors,
    }
}

fn apply_replacements(logical: &str, repl: &[(usize, usize, String)]) -> String {
    let mut ordered = repl.to_vec();
    ordered.sort_by_key(|(s, _, _)| *s);
    let mut out = String::new();
    let mut cursor = 0;
    for (start, end, rep) in ordered {
        if start < cursor {
            continue; // overlap skip
        }
        out.push_str(&logical[cursor..start]);
        out.push_str(&rep);
        cursor = end;
    }
    out.push_str(&logical[cursor..]);
    out
}

fn read_bracket_entity(s: &str, start: usize) -> Result<(usize, String, Option<String>), usize> {
    // start 指向 `@[` / `&[` / `#[` / `$[` 的前缀字符
    let marker_len = s[start..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
    let open = start + marker_len + 1; // 跳过 marker + '['
    if open > s.len() {
        return Err(s.len());
    }
    // 实体须在同一行闭合（SPEC：name 内不含换行）
    let line_end = s[open..].find('\n').map(|n| open + n).unwrap_or(s.len());
    let line_rest = &s[open..line_end];
    let Some(close) = line_rest.find(']') else {
        return Err(line_end);
    };
    let inner = &line_rest[..close];
    let end = open + close + 1;
    let (name, id) = if let Some((n, i)) = inner.split_once('|') {
        (n.to_string(), Some(i.to_string()))
    } else {
        (inner.to_string(), None)
    };
    if name.is_empty() {
        return Err(end);
    }
    Ok((end, name, id))
}

fn match_class_span(s: &str, start: usize) -> Option<(usize, String, String)> {
    // 形式：[text]{.class}
    let rest = &s[start + 1..];
    let close = rest.find(']')?;
    let text = &rest[..close];
    let after = &rest[close + 1..];
    if !after.starts_with("{.") {
        return None;
    }
    let inner = &after[2..];
    let end_brace = inner.find('}')?;
    let class = &inner[..end_brace];
    if class.is_empty() || class.chars().any(|c| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
    {
        return None;
    }
    let end = start + 1 + close + 1 + 2 + end_brace + 1;
    Some((end, text.to_string(), class.to_string()))
}

fn match_color_span(s: &str, start: usize) -> Option<(usize, String, String)> {
    // 形式：==text=={color=...}
    if !s[start..].starts_with("==") {
        return None;
    }
    let rest = &s[start + 2..];
    let mid = rest.find("==")?;
    let text = &rest[..mid];
    let after = &rest[mid + 2..];
    if !after.starts_with("{color=") {
        return None;
    }
    let inner = &after["{color=".len()..];
    let end_brace = inner.find('}')?;
    let color = &inner[..end_brace];
    let end = start + 2 + mid + 2 + "{color=".len() + end_brace + 1;
    Some((end, text.to_string(), color.to_string()))
}

fn match_revision_ref(s: &str, start: usize) -> Option<(usize, String)> {
    let rest = &s[start..];
    if !rest.starts_with("[^c:") {
        return None;
    }
    let inner = &rest[4..];
    let close = inner.find(']')?;
    let id = &inner[..close];
    if id.is_empty() {
        return None;
    }
    Some((start + 4 + close + 1, id.to_string()))
}

fn match_footnote_ref(s: &str, start: usize) -> Option<(usize, String)> {
    let rest = &s[start..];
    if !rest.starts_with("[^") {
        return None;
    }
    let inner = &rest[2..];
    let close = inner.find(']')?;
    let label = &inner[..close];
    if label.is_empty() {
        return None;
    }
    // `c:` 前缀由修订语法处理；此处收集普通脚注引用
    Some((start + 2 + close + 1, label.to_string()))
}

fn match_revision_def(s: &str, line_start: usize) -> Option<(usize, String, String)> {
    let line = s[line_start..].split('\n').next()?;
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    if indent > 3 {
        return None;
    }
    if !trimmed.starts_with("[^c:") {
        return None;
    }
    let after = &trimmed[4..];
    let close = after.find(']')?;
    let id = &after[..close];
    let rest = &after[close + 1..];
    if !rest.starts_with(':') {
        return None;
    }
    let content = rest[1..].trim_start().to_string();
    let end = line_start + line.len();
    Some((end, id.to_string(), content))
}

fn match_footnote_def(s: &str, line_start: usize) -> Option<(usize, String, String)> {
    let line = s[line_start..].split('\n').next()?;
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    if indent > 3 {
        return None;
    }
    if !trimmed.starts_with("[^") || trimmed.starts_with("[^c:") {
        return None;
    }
    let after = &trimmed[2..];
    let close = after.find(']')?;
    let label = &after[..close];
    let rest = &after[close + 1..];
    if !rest.starts_with(':') {
        return None;
    }
    let content = rest[1..].trim_start().to_string();
    let end = line_start + line.len();
    Some((end, label.to_string(), content))
}

fn match_anchor(s: &str, start: usize) -> Option<(usize, String)> {
    let rest = &s[start..];
    if !rest.starts_with("{#") {
        return None;
    }
    let inner = &rest[2..];
    let close = inner.find('}')?;
    let id = &inner[..close];
    if id.is_empty()
        || !id.chars().next()?.is_ascii_alphabetic()
        || id
            .chars()
            .any(|c| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
    {
        return None;
    }
    Some((start + 2 + close + 1, id.to_string()))
}
