//! 扫描逻辑正文中的 JMD 扩展，并生成带占位符的渲染正文。
//!
//! ```text
//! frontmatter → soft_join → scan_extensions（本模块）→ markdown 渲染
//! ```
//!
//! 产出：AST 节点（Span 已映回源文档）+ `render_body`（扩展换成 `%%JMD_*%%`，
//! 禁止的代码语法由 [`super::forbid_code`] 识别后在此抹空）。
//!
//! # 匹配优先级（行首块级优先）
//!
//! 围栏 → 修订/脚注定义 → 行内代码 → 实体 → 类名染色（opener 栈）→
//! 修订引用 → 脚注引用 → 锚点
//!
//! # 嵌套规则
//!
//! - **染色**可嵌套，且可包裹实体 / 脚注引用 / 修订引用 / 锚点 / 内层染色；
//! - 实体、脚注、修订、锚点为**叶子**，其内部不再解析 JMD 扩展。
//! - 染色配对见 [`super::style_stack`]（局部 opener 栈，O(N)）。

use super::forbid_code::{match_fenced_code, match_inline_code, CodeHit};
use super::style_stack::{apply_replacements, StyleStack};
use super::util::{
    at_bol, char_at, include_trailing_nl, indent_ok_cm, is_id_char, next_char, same_line,
};
use crate::analyzer::Diagnostic;
use crate::ast::{
    Anchor, Entity, EntityKind, Footnote, FootnoteKind, RevisionDef, RevisionRef, Span, StyledSpan,
};
use crate::placeholder::Placeholder;

/// 一次扩展扫描的全部产物。
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

// ─── 偏移映射 ───────────────────────────────────────────────

/// `logical` 字节 → 源文档绝对 Span（经 soft_join 表 + frontmatter 基准）。
struct Mapper<'a> {
    body_byte_offset: usize,
    logic_to_body: &'a [usize],
}

impl Mapper<'_> {
    fn span(&self, start: usize, end: usize) -> Span {
        let b0 = self.logic_to_body.get(start).copied().unwrap_or(start);
        let abs0 = self.body_byte_offset + b0;
        if end <= start {
            return Span::new(abs0, abs0);
        }
        // 半开区间：优先用 end 处映射（下一逻辑字节的 body 起点）；越界则末字节 +1
        let b1 = if end < self.logic_to_body.len() {
            self.logic_to_body[end]
        } else {
            self.logic_to_body
                .get(end - 1)
                .copied()
                .unwrap_or(b0)
                .saturating_add(1)
        };
        Span::new(abs0, self.body_byte_offset + b1)
    }
}

// ─── 扫描状态机 ─────────────────────────────────────────────

/// 主循环可变状态；`wipe` / `emit` 统一推进指针与行首标记。
struct ScanState<'a> {
    logical: &'a str,
    map: Mapper<'a>,
    entities: Vec<Entity>,
    anchors: Vec<Anchor>,
    styled_spans: Vec<StyledSpan>,
    revision_refs: Vec<RevisionRef>,
    revision_defs: Vec<RevisionDef>,
    footnotes: Vec<Footnote>,
    errors: Vec<Diagnostic>,
    /// 顶层 `(start, end, replacement)`；染色帧内替换在 [`StyleStack`]。
    repl: Vec<(usize, usize, String)>,
    style_stack: StyleStack,
    i: usize,
    bol: bool,
}

impl<'a> ScanState<'a> {
    fn new(logical: &'a str, logic_to_body: &'a [usize], body_byte_offset: usize) -> Self {
        Self {
            logical,
            map: Mapper {
                body_byte_offset,
                logic_to_body,
            },
            entities: Vec::new(),
            anchors: Vec::new(),
            styled_spans: Vec::new(),
            revision_refs: Vec::new(),
            revision_defs: Vec::new(),
            footnotes: Vec::new(),
            errors: Vec::new(),
            repl: Vec::new(),
            style_stack: StyleStack::new(),
            i: 0,
            bol: true,
        }
    }

    fn wipe(&mut self, start: usize, end: usize) {
        self.emit_repl(start, end, String::new());
        self.i = end;
        self.bol = at_bol(self.logical, end);
    }

    fn emit(&mut self, start: usize, end: usize, rep: String) {
        self.emit_repl(start, end, rep);
        self.i = end;
        self.bol = false;
    }

    fn emit_repl(&mut self, start: usize, end: usize, rep: String) {
        if self.style_stack.is_empty() {
            self.repl.push((start, end, rep));
        } else {
            let _ = self.style_stack.put(start, end, rep);
        }
    }

    fn bump_char(&mut self) {
        let ch = char_at(self.logical, self.i).unwrap_or('\0');
        self.bol = ch == '\n';
        self.i = next_char(self.logical, self.i);
    }

    /// 换行或即将结束：放弃未闭合染色帧，叶子替换上提。
    fn abandon_open_styles(&mut self) {
        self.style_stack.clear_into(&mut self.repl);
    }

    /// 禁止代码：写诊断并抹除。
    fn reject_code(&mut self, start: usize, hit: CodeHit, closed_msg: &str, open_msg: &str) {
        let (end, diag) = match hit {
            CodeHit::Closed { end } => (
                end,
                Diagnostic::warning(closed_msg, Some(self.map.span(start, end))),
            ),
            CodeHit::Unclosed { end } => (
                end,
                Diagnostic::error(open_msg, Some(self.map.span(start, end))),
            ),
        };
        self.errors.push(diag);
        self.wipe(start, end);
    }

    fn finish(mut self) -> ScanResult {
        self.abandon_open_styles();
        ScanResult {
            render_body: apply_replacements(self.logical, &self.repl),
            entities: self.entities,
            anchors: self.anchors,
            styled_spans: self.styled_spans,
            revision_refs: self.revision_refs,
            revision_defs: self.revision_defs,
            footnotes: self.footnotes,
            errors: self.errors,
        }
    }
}

/// 单遍扫描 `logical`，收集 AST 与替换区间。
pub fn scan_extensions(
    logical: &str,
    logic_to_body: &[usize],
    body_byte_offset: usize,
) -> ScanResult {
    let mut s = ScanState::new(logical, logic_to_body, body_byte_offset);

    while s.i < s.logical.len() {
        if !s.logical.is_char_boundary(s.i) {
            s.i += 1;
            continue;
        }

        // —— 行首块级 ——
        if s.bol {
            if let Some(hit) = match_fenced_code(s.logical, s.i) {
                s.reject_code(
                    s.i,
                    hit,
                    "JMD 正文不允许代码围栏",
                    "未闭合的代码围栏",
                );
                continue;
            }
            if let Some((end, id, content)) = match_revision_def(s.logical, s.i) {
                s.revision_defs.push(RevisionDef {
                    id,
                    content,
                    span: s.map.span(s.i, end),
                });
                s.wipe(s.i, include_trailing_nl(s.logical, end));
                continue;
            }
            if let Some((end, label)) = match_footnote_def(s.logical, s.i) {
                s.footnotes.push(Footnote {
                    label,
                    kind: FootnoteKind::Def,
                    span: s.map.span(s.i, end),
                });
                // 原文留给 markdown-rs；只跳过 `[`，避免重复当定义处理
                s.i = next_char(s.logical, s.i);
                s.bol = false;
                continue;
            }
        }

        // —— 转义：\` 下一字符不定界（行尾 `\` 只跳过自身）——
        if s.logical.as_bytes().get(s.i) == Some(&b'\\') {
            s.i += 1;
            if s.i < s.logical.len() && s.logical.as_bytes()[s.i] != b'\n' {
                s.i = next_char(s.logical, s.i);
            }
            s.bol = false;
            continue;
        }

        // —— 行内禁止代码 ——
        if s.logical[s.i..].starts_with('`') {
            if let Some(hit) = match_inline_code(s.logical, s.i) {
                s.reject_code(
                    s.i,
                    hit,
                    "JMD 正文不允许行内代码",
                    "未闭合的行内代码",
                );
                continue;
            }
        }

        let ch = char_at(s.logical, s.i).unwrap_or('\0');

        // —— 换行：放弃本行未闭合染色 ——
        if ch == '\n' {
            s.abandon_open_styles();
            s.bump_char();
            continue;
        }

        // —— 实体 @[…] ——
        if let Some(kind) = EntityKind::from_marker(ch) {
            if s.logical[s.i + ch.len_utf8()..].starts_with('[') {
                match read_bracket_entity(s.logical, s.i) {
                    Ok((end, name, entity_id, name_start, name_end)) => {
                        let idx = s.entities.len();
                        s.entities.push(Entity {
                            kind,
                            name,
                            entity_id,
                            span: s.map.span(s.i, end),
                            name_span: s.map.span(name_start, name_end),
                        });
                        s.emit(s.i, end, Placeholder::Entity.format(idx));
                    }
                    Err(()) => {
                        let mark_end = s.i + ch.len_utf8() + 1;
                        s.errors.push(Diagnostic::error(
                            "未闭合的实体标注",
                            Some(s.map.span(s.i, mark_end)),
                        ));
                        s.i = next_char(s.logical, s.i);
                        s.bol = false;
                    }
                }
                continue;
            }
        }

        // —— 修订引用（先于普通脚注；须在裸 `[` 入栈之前）——
        if s.logical[s.i..].starts_with("[^c:") {
            if let Some((end, id)) = match_revision_ref(s.logical, s.i) {
                let idx = s.revision_refs.len();
                s.revision_refs.push(RevisionRef {
                    id,
                    span: s.map.span(s.i, end),
                });
                s.emit(s.i, end, Placeholder::Revision.format(idx));
                continue;
            }
        }

        // —— 普通脚注引用（原文保留） ——
        if s.logical[s.i..].starts_with("[^") {
            if let Some((end, label)) = match_footnote_ref(s.logical, s.i) {
                s.footnotes.push(Footnote {
                    label,
                    kind: FootnoteKind::Ref,
                    span: s.map.span(s.i, end),
                });
                s.i = end;
                s.bol = false;
                continue;
            }
        }

        // —— 锚点 ——
        if s.logical[s.i..].starts_with("{#") {
            if let Some((end, id)) = match_anchor(s.logical, s.i) {
                s.anchors.push(Anchor {
                    id,
                    span: s.map.span(s.i, end),
                });
                let idx = s.anchors.len() - 1;
                s.emit(s.i, end, Placeholder::Anchor.format(idx));
                continue;
            }
        }

        // —— 染色 opener ——
        if ch == '[' {
            s.style_stack.push_open(s.i);
            s.i += 1;
            s.bol = false;
            continue;
        }

        // —— 染色 closer：`]{.class}` ——
        if ch == ']' {
            let line = same_line(s.logical, s.i + 1);
            if let Some((class, braced)) = take_braced(line, "{.") {
                if !class.is_empty()
                    && class.chars().all(is_id_char)
                    && !s.style_stack.is_empty()
                {
                    if let Some((open, text)) = s.style_stack.try_close(s.i, s.logical) {
                        let end = s.i + 1 + braced;
                        let idx = s.styled_spans.len();
                        s.styled_spans.push(StyledSpan {
                            text,
                            class: class.to_string(),
                            span: s.map.span(open, end),
                        });
                        s.emit(open, end, Placeholder::Style.format(idx));
                        continue;
                    }
                }
            }
        }

        s.bump_char();
    }

    s.finish()
}

// ─── 扩展语法匹配 ───────────────────────────────────────────

/// `@[可见名|id]`；须同行闭合，空名失败。
/// 成功时返回 `(end, name, id, name_start, name_end)`（logical 字节下标）。
fn read_bracket_entity(
    s: &str,
    start: usize,
) -> Result<(usize, String, Option<String>, usize, usize), ()> {
    let marker_len = char_at(s, start).map(|c| c.len_utf8()).unwrap_or(1);
    let open = start + marker_len + 1;
    if open > s.len() {
        return Err(());
    }
    let line = same_line(s, open);
    let close = line.find(']').ok_or(())?;
    let inner = &line[..close];
    let end = open + close + 1;
    let (name, id) = match inner.split_once('|') {
        Some((n, i)) => (n.to_string(), Some(i.to_string())),
        None => (inner.to_string(), None),
    };
    if name.is_empty() {
        return Err(());
    }
    let name_start = open;
    let name_end = open + name.len();
    Ok((end, name, id, name_start, name_end))
}

/// 非空修订 id：`c:rev_01` → `rev_01`。
fn revision_id(label: &str) -> Option<&str> {
    label.strip_prefix("c:").filter(|id| !id.is_empty())
}

/// 行首 `[^label]: content` → `(行末, label, content)`。
fn match_caret_def(s: &str, line_start: usize) -> Option<(usize, &str, &str)> {
    let line = same_line(s, line_start);
    if !indent_ok_cm(line) {
        return None;
    }
    let trimmed = line.trim_start();
    if !trimmed.starts_with("[^") {
        return None;
    }
    let after = &trimmed[2..];
    let close = after.find(']')?;
    let label = &after[..close];
    if label.is_empty() {
        return None;
    }
    let rest = &after[close + 1..];
    if !rest.starts_with(':') {
        return None;
    }
    Some((line_start + line.len(), label, rest[1..].trim_start()))
}

fn match_revision_def(s: &str, line_start: usize) -> Option<(usize, String, String)> {
    let (end, label, content) = match_caret_def(s, line_start)?;
    Some((end, revision_id(label)?.to_string(), content.to_string()))
}

fn match_footnote_def(s: &str, line_start: usize) -> Option<(usize, String)> {
    let (end, label, _) = match_caret_def(s, line_start)?;
    if label.starts_with("c:") {
        return None;
    }
    Some((end, label.to_string()))
}

/// 行内 `[^…]` → `(end, label)`。
fn match_caret_ref(s: &str, start: usize) -> Option<(usize, &str)> {
    let line = same_line(s, start);
    if !line.starts_with("[^") {
        return None;
    }
    let inner = &line[2..];
    let close = inner.find(']')?;
    let label = &inner[..close];
    if label.is_empty() {
        return None;
    }
    Some((start + 2 + close + 1, label))
}

fn match_revision_ref(s: &str, start: usize) -> Option<(usize, String)> {
    let (end, label) = match_caret_ref(s, start)?;
    Some((end, revision_id(label)?.to_string()))
}

fn match_footnote_ref(s: &str, start: usize) -> Option<(usize, String)> {
    let (end, label) = match_caret_ref(s, start)?;
    if label.starts_with("c:") {
        return None;
    }
    Some((end, label.to_string()))
}

/// `{prefix … }` → `(inner, 含闭括号的消费长度)`。
fn take_braced<'a>(rest: &'a str, prefix: &str) -> Option<(&'a str, usize)> {
    let after = rest.strip_prefix(prefix)?;
    let end = after.find('}')?;
    Some((&after[..end], prefix.len() + end + 1))
}

fn match_anchor(s: &str, start: usize) -> Option<(usize, String)> {
    let line = same_line(s, start);
    let inner = line.strip_prefix("{#")?;
    let close = inner.find('}')?;
    let id = &inner[..close];
    let mut chars = id.chars();
    let first = chars.next()?;
    if !first.is_ascii_alphabetic() || !chars.all(is_id_char) {
        return None;
    }
    Some((start + 2 + close + 1, id.to_string()))
}

// ─── 测试 ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(logical: &str) -> ScanResult {
        let map: Vec<_> = (0..logical.len()).collect();
        scan_extensions(logical, &map, 0)
    }

    fn has_msg(r: &ScanResult, part: &str) -> bool {
        r.errors.iter().any(|d| d.message.contains(part))
    }

    // 围栏命中后由 scan 抹除并写入 Warning（匹配细节见 forbid_code 单测）
    #[test]
    fn closed_fence_wiped_with_warning() {
        let r = scan("前文\n```rust\nlet x = 1;\n```\n后文\n");
        assert!(r.render_body.contains("前文") && r.render_body.contains("后文"));
        assert!(!r.render_body.contains("```") && !r.render_body.contains("let x"));
        assert!(has_msg(&r, "不允许代码围栏"));
    }

    #[test]
    fn unclosed_fence_is_error() {
        let r = scan("前文\n```\n还在里面\n");
        assert!(has_msg(&r, "未闭合的代码围栏"));
        assert!(!r.render_body.contains("```"));
    }

    #[test]
    fn closed_inline_code_wiped_with_warning() {
        let r = scan("见 `code` 处\n");
        assert!(!r.render_body.contains('`') && !r.render_body.contains("code"));
        assert!(has_msg(&r, "不允许行内代码"));
    }

    // 类名染色不得跨行
    #[test]
    fn class_span_does_not_cross_newline() {
        assert!(scan("[未闭合\n]{.note}\n").styled_spans.is_empty());
    }

    // 染色包裹实体：外层 class + 内层 Entity
    #[test]
    fn style_wrapping_entity() {
        let r = scan("[&[南阳|ny]]{.note}\n");
        assert_eq!(r.styled_spans.len(), 1);
        assert_eq!(r.styled_spans[0].class, "note");
        assert_eq!(r.entities.len(), 1);
        assert_eq!(r.entities[0].name, "南阳");
        assert!(r.styled_spans[0].text.contains("%%JMD_E_"));
        assert!(r.render_body.contains("%%JMD_S_"));
    }

    // 染色自嵌套
    #[test]
    fn nested_styles() {
        let r = scan("[[内]{.em}外]{.note}\n");
        assert_eq!(r.styled_spans.len(), 2);
        let classes: Vec<_> = r.styled_spans.iter().map(|s| s.class.as_str()).collect();
        assert!(classes.contains(&"em"));
        assert!(classes.contains(&"note"));
    }

    // 转义 \] 不定界
    #[test]
    fn style_escape_close_bracket() {
        let r = scan("[a\\]b]{.red}\n");
        assert_eq!(r.styled_spans.len(), 1);
        assert_eq!(r.styled_spans[0].class, "red");
        assert!(r.styled_spans[0].text.contains(']'));
    }

    // 许多未闭合 `[` 不产生染色，且 O(N) 可完成
    #[test]
    fn many_unclosed_brackets_no_style() {
        let s = "[".repeat(200) + "\n";
        let r = scan(&s);
        assert!(r.styled_spans.is_empty());
    }

    // 未闭合染色内的实体仍上提为占位
    #[test]
    fn abandoned_style_still_emits_entity() {
        let r = scan("[见@[光武|x]\n");
        assert!(r.styled_spans.is_empty());
        assert_eq!(r.entities.len(), 1);
        assert!(r.render_body.contains("%%JMD_E_"));
        assert!(r.render_body.contains('['));
    }

    // 空 label / 空修订 id 不识别
    #[test]
    fn empty_def_labels_rejected() {
        let r = scan("[^]: 空脚注\n[^c:]: 空修订\n正文\n");
        assert!(r.footnotes.is_empty() && r.revision_defs.is_empty());
        assert!(r.render_body.contains("[^]:") && r.render_body.contains("[^c:]:"));
    }

    // 连续修订定义抹除含 \n，文首不留空段
    #[test]
    fn consecutive_revision_defs_do_not_leave_blank_paragraph() {
        let r = scan("[^c:1]: 一\n[^c:2]: 二\n后文\n");
        assert_eq!(r.revision_defs.len(), 2);
        assert!(!r.render_body.contains("[^c:"));
        assert!(r.render_body.starts_with("后文"));
    }
}
