//! 染色定界符栈（局部 opener 栈）。
//!
//! 算法借鉴 Djot 行内容器匹配，参考 jotdown `src/inline.rs` 的 `openers` 栈
//!（见 <https://github.com/hellux/jotdown> — `Opener` push / closer 时匹配 drain）。
//! 此处收窄为 JMD `[text]{.class}` 嵌套，不引入 Djot lexer/event 模型。
//!
//! 与上游差异：帧内携带局部 `repl`，闭合时生成已嵌入 `%%JMD_*%%` 的渲染片段；
//! 行末或 EOF 放弃未闭合帧时，将帧内叶子替换上提到外层 / 顶层。

/// 一个尚未闭合的 `[` 及其后写入的局部替换。
struct StyleFrame {
    /// `[` 的 logical 字节下标。
    open: usize,
    /// 本帧打开后收集的替换（绝对区间，按扫描顺序）。
    repl: Vec<(usize, usize, String)>,
}

/// 染色 opener 栈；由 [`super::scan`] 在单遍主循环中驱动。
#[derive(Default)]
pub struct StyleStack {
    frames: Vec<StyleFrame>,
}

impl StyleStack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// 见 `[`：压入潜在染色 opener（不向前探测）。
    pub fn push_open(&mut self, open: usize) {
        self.frames.push(StyleFrame {
            open,
            repl: Vec::new(),
        });
    }

    /// 叶子或内层 Style 占位写入当前帧；无帧返回 `false`（由调用方写入顶层 `repl`）。
    pub fn put(&mut self, start: usize, end: usize, rep: String) -> bool {
        match self.frames.last_mut() {
            Some(frame) => {
                frame.repl.push((start, end, rep));
                true
            }
            None => false,
        }
    }

    /// `]` 闭合栈顶：返回 `(open, 已应用帧内替换的 content)`。
    pub fn try_close(&mut self, close: usize, logical: &str) -> Option<(usize, String)> {
        let frame = self.frames.pop()?;
        let from = frame.open + 1;
        if close < from || close > logical.len() {
            return None;
        }
        let relative: Vec<(usize, usize, String)> = frame
            .repl
            .into_iter()
            .map(|(s, e, r)| (s - from, e - from, r))
            .collect();
        Some((
            frame.open,
            apply_replacements(&logical[from..close], &relative),
        ))
    }

    /// 放弃未闭合帧，把帧内已收集的叶子替换并入 `top_repl`（原文 `[` 等保留）。
    pub fn clear_into(&mut self, top_repl: &mut Vec<(usize, usize, String)>) {
        for frame in self.frames.drain(..) {
            top_repl.extend(frame.repl);
        }
    }
}

/// 按有序不重叠区间把 `repl` 应用到 `logical`。
pub fn apply_replacements(logical: &str, repl: &[(usize, usize, String)]) -> String {
    let mut out = String::with_capacity(logical.len());
    let mut cursor = 0;
    for (start, end, rep) in repl {
        debug_assert!(
            *start >= cursor,
            "replacement ranges must be ordered and non-overlapping"
        );
        if *start < cursor {
            continue;
        }
        out.push_str(&logical[cursor..*start]);
        out.push_str(rep);
        cursor = *end;
    }
    out.push_str(&logical[cursor..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_without_frame_returns_false() {
        let mut s = StyleStack::new();
        assert!(!s.put(0, 1, "x".into()));
        assert!(s.is_empty());
    }

    #[test]
    fn nested_close_order() {
        let logical = "[[in]{.em}out]";
        // open0=0 `[`, open1=1 `[`, close inner `]` at 4
        let mut s = StyleStack::new();
        s.push_open(0);
        s.push_open(1);
        let (open, text) = s.try_close(4, logical).unwrap();
        assert_eq!(open, 1);
        assert_eq!(text, "in");
        assert!(!s.is_empty());
        let (open, text) = s.try_close(logical.len() - 1, logical).unwrap();
        assert_eq!(open, 0);
        assert!(text.contains("in") && text.contains("out"));
    }

    /// 三层：`[[[a]{.c0}b]{.c1}c]{.c2}`，自内向外关闭并写入占位。
    #[test]
    fn triple_nested_close_with_placeholders() {
        //  indices: 0[ 1[ 2[ 3a 4] … mid … outer]
        let logical = "[[[a]{.c0}b]{.c1}c]";
        let mut s = StyleStack::new();
        s.push_open(0);
        s.push_open(1);
        s.push_open(2);

        let (open, text) = s.try_close(4, logical).unwrap();
        assert_eq!((open, text.as_str()), (2, "a"));
        assert!(s.put(2, 10, "%%S0%%".into())); // `[a]{.c0}` → 占位

        let (open, text) = s.try_close(11, logical).unwrap();
        assert_eq!(open, 1);
        assert_eq!(text, "%%S0%%b");
        assert!(s.put(1, 17, "%%S1%%".into())); // `[…]{.c1}`

        let (open, text) = s.try_close(logical.len() - 1, logical).unwrap();
        assert_eq!(open, 0);
        assert_eq!(text, "%%S1%%c");
        assert!(s.is_empty());
    }

    #[test]
    fn clear_into_flushes_leaf_repl() {
        let mut s = StyleStack::new();
        s.push_open(0);
        assert!(s.put(2, 5, "%%E%%".into()));
        let mut top = Vec::new();
        s.clear_into(&mut top);
        assert!(s.is_empty());
        assert_eq!(top, vec![(2, 5, "%%E%%".into())]);
    }
}
