//! scan → render 共用的扩展占位符格式，避免两边字符串漂移。

/// 占位符种类（与 `%%JMD_X_n%%` 中的字母对应）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Placeholder {
    Entity,
    Style,
    Revision,
    Anchor,
}

impl Placeholder {
    pub(crate) fn format(self, index: usize) -> String {
        let tag = match self {
            Self::Entity => 'E',
            Self::Style => 'S',
            Self::Revision => 'R',
            Self::Anchor => 'A',
        };
        format!("%%JMD_{tag}_{index}%%")
    }
}
