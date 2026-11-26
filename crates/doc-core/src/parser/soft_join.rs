use unicode_script::{Script, UnicodeScript};

use crate::ast::EntityKind;

/// 行尾 `\` 软分段合并后的结果。
pub struct SoftJoinResult {
    pub logical: String,
    /// `logic_to_body[i]`：`logical` 第 i 字节对应原始 `body` 中的字节偏移。
    pub logic_to_body: Vec<usize>,
}

fn is_han(c: char) -> bool {
    c.script() == Script::Han
}

/// 软拼接时把中文标点视作紧贴字符（换行不加空格）。
fn is_cjk_for_soft_join(c: char) -> bool {
    is_han(c)
        || matches!(
            c,
            '\u{3001}'..='\u{303F}' // CJK 标点（、。〈〉《》「」『』【】等；排除 \u{3000} 全角空格）
                | '\u{FE10}'..='\u{FE1F}' // 竖排标点
                | '\u{FE30}'..='\u{FE4F}' // CJK 兼容形式（竖排括号等）
                | '\u{FF01}'..='\u{FF0F}' // 全角标点 ！＂＃＄％＆＇（）＊＋，－．／
                | '\u{FF1A}'..='\u{FF20}' // 全角标点 ：；＜＝＞？＠
                | '\u{FF3B}'..='\u{FF40}' // 全角标点 ［＼］＾＿｀
                | '\u{FF5B}'..='\u{FF65}' // 全角标点 ｛｜｝～ 及半角句号逗号
                | '·' // 间隔号（中文译名常用，如 阿尔伯特·爱因斯坦）
        )
}

/// 若行尾为软分段符 `\`，返回去掉 `\` 及尾随空白后的内容。
fn strip_soft_join(line: &str) -> Option<&str> {
    let trimmed = line.trim_end();
    if !trimmed.ends_with('\\') {
        return None;
    }
    let without = &trimmed[..trimmed.len() - 1];
    // `\\` 行尾不是软分段符
    if without.ends_with('\\') {
        return None;
    }
    Some(without)
}

fn push_mapped(logical: &mut String, map: &mut Vec<usize>, text: &str, body_base: usize) {
    for (i, _) in text.as_bytes().iter().enumerate() {
        map.push(body_base + i);
    }
    logical.push_str(text);
}

fn push_char_at(logical: &mut String, map: &mut Vec<usize>, ch: char, body_off: usize) {
    let mut buf = [0u8; 4];
    let s = ch.encode_utf8(&mut buf);
    for _ in s.as_bytes() {
        map.push(body_off);
    }
    logical.push_str(s);
}

/// 合并以 `\` 结尾的行，并丢掉其后空行，得到供后续解析的逻辑正文流。
pub fn soft_join(body: &str) -> SoftJoinResult {
    let mut logical = String::with_capacity(body.len());
    let mut logic_to_body = Vec::with_capacity(body.len());

    // ① 行分割处理
    let body_addr = body.as_ptr() as usize;
    let lines: Vec<(usize, &str)> = body
        .lines()
        .map(|line| (line.as_ptr() as usize - body_addr, line))
        .collect();

    let mut i = 0;
    let mut mid_paragraph = false;

    while i < lines.len() {
        let (start, line) = lines[i];

        // ② 软分段符处理
        let (content, joining) = match strip_soft_join(line) {
            Some(content) => (content, true),
            None => (line, false),
        };

        // ③ 空行处理
        if !joining && line.is_empty() {
            if mid_paragraph {
                // 无软分段符的空行：结束当前逻辑段
                mid_paragraph = false;
            }
            push_char_at(&mut logical, &mut logic_to_body, '\n', start);
            i += 1;
            continue;
        }

        // ④ 逻辑段衔接处理
        if mid_paragraph && !content.is_empty() {
            // CJK 两侧、或下一句以实体标注开头 → 不加空格；其余插 ASCII 空格
            let glue = match (logical.chars().next_back(), content.chars().next()) {
                (Some(a), Some(b)) if is_cjk_for_soft_join(a) && is_cjk_for_soft_join(b) => None,
                (Some(_), Some(_)) if EntityKind::starts_at(content) => None,
                (Some(_), Some(_)) => Some((' ', start)),
                _ => None,
            };
            if let Some((ch, off)) = glue {
                push_char_at(&mut logical, &mut logic_to_body, ch, off);
            }
        }

        push_mapped(&mut logical, &mut logic_to_body, content, start);
        i += 1;

        if joining {
            // 有软分段符 → 进入新的逻辑段
            mid_paragraph = true;
            // 软分段符后丢弃全部空行，并与下一非空行拼入同一段
            while i < lines.len() && lines[i].1.trim().is_empty() {
                i += 1;
            }
        } else {
            // 无软分段符 → 结束当前逻辑段
            mid_paragraph = false;
            if i < lines.len() {
                // 后面还有行 ⇒ 本行后必有换行符，偏移即 start + line.len()
                push_char_at(&mut logical, &mut logic_to_body, '\n', start + line.len());
            }
        }
    }

    SoftJoinResult {
        logical,
        logic_to_body,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_with_blank_lines() {
        let body = "甲乙，\\\n\n丙丁。\n";
        let SoftJoinResult { logical, .. } = soft_join(body);
        assert_eq!(logical.trim_end(), "甲乙，丙丁。");
    }

    #[test]
    fn soft_join_skips_multiple_blank_lines() {
        let body = "甲\\\n\n\n乙\n";
        let SoftJoinResult { logical, .. } = soft_join(body);
        assert_eq!(logical.trim_end(), "甲乙");
    }

    #[test]
    fn real_paragraph_break_without_backslash() {
        let body = "第一段。\n\n第二段。\n";
        let SoftJoinResult { logical, .. } = soft_join(body);
        assert!(logical.contains("第一段。\n\n第二段。"));
    }

    #[test]
    fn escaped_backslash_is_not_soft_join() {
        let body = "path\\\\ \nnext\n";
        let SoftJoinResult { logical, .. } = soft_join(body);
        assert!(logical.contains('\\'));
        assert!(logical.contains('\n'));
    }
}
