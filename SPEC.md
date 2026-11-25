# JMD 语言规范（v0.5）

本文档是 **JMD**（文件扩展名 `.jmd`，语言 ID `jmd`）的规范性说明。  
实现（`doc-core` / `doc-lsp` / `doc-wasm`）与示例必须以本文为准；与 [MVP.md](MVP.md) 冲突时，以本 SPEC 为准。

---

## 1. 文档地位与目标

JMD 面向 **历史文献转写 / 整理**：在可读正文中标注人物、地点、时间、官职等实体，并支持消歧与超链；同时保留审阅评论、结构锚点与 Frontmatter 等工程能力。

JMD 建立在 **CommonMark + GFM** 之上，本版本能力分层如下：

**核心（历史转写）**

1. **实体标注 + 消歧 + 超链**（仅括号形）：`@[…]` 人名、`&[…]` 地名、`#[…]` 时间/年号、`$[…]` 官职  

**配套**

2. **软分段符**（写作时一句一行 / 可空行；编译合并为同一段）  
3. 结构锚点 `{#id}`（文档内定位；站内跳转使用标准 Markdown 链接）  
4. 文本染色（class / color）  
5. GFM 标准脚注（须支持点击跳转）  
6. 侧栏修订标记（语法预留，行为后续补充）  
7. TOML Frontmatter  

**本版本明确不包含**：裸实体（如 `@李白`）、Wikilink（`[[…]]`）、侧栏修订的完整审阅 UI/数据协议、自动 NER、数学公式（`$…$` 关闭，避免与官职前缀冲突）。

---

## 2. 术语

| 术语 | 含义 |
| --- | --- |
| **源文档** | 完整 `.jmd` 文本，含可选 Frontmatter |
| **正文** | 剥离 Frontmatter 后的主体 |
| **实体（Entity）** | 人名 / 地名 / 时间 / 官职等历史语义对象 |
| **实体 ID** | 知识库或权威库中的消歧键（可为数字或字符串） |
| **Span** | 半开区间 `[start, end)`，**UTF-8 字节偏移**，相对整份源文档 |
| **行号** | 1-based，相对整份源文档 |
| **符号表** | 结构锚点 ID → 定义位置（单文件） |
| **软分段符** | 写作时可见的「段内续行」标记；编译时删除并与后文拼成同一 HTML 段落 |
| **诊断** | Error / Warning / Info |

---

## 3. 处理管线（规范顺序）

```text
源文档
  → ① 剥离并解析 Frontmatter（记录 body_byte_offset）
  → ② 软分段合并（去掉行尾 `\`，折叠其后空行，拼成逻辑正文）
  → ③ 扫描 JMD 扩展（实体 → 修订标记 → 染色 → 结构锚点）
  → ④ CommonMark/GFM 解析正文（含标准脚注；关闭数学）
  → ⑤ 合入渲染（实体标签 / HTML / 脚注双向锚点）
  → ⑥ 结构符号表 + 脚注闭环校验
  → ⑦ 输出 Document + RenderOutput + Diagnostics
```

> **Span 注意**：诊断与编辑器定位仍相对**原始源文档**。软分段是渲染/逻辑正文的变换；实现须保留「逻辑偏移 → 源偏移」映射，或在 AST 节点上继续使用合并前的 Span。

**冲突优先级（同区间重叠时，先匹配者优先）**：

1. 脚注 / 修订定义行 `[^…]:`（块级整行）  
2. 修订引用 `[^c:…]`（先于普通脚注识别）  
3. 括号形实体 `@[…]` / `&[…]` / `#[…]` / `$[…]`  
4. 染色 `[text]{.class}` / `==text=={color=…}`  
5. 结构锚点 `{#id}`  
6. 普通 Markdown / GFM（含标准脚注 `[^label]`）  

代码围栏与行内代码内扩展语法 **不生效**。软分段符在围栏内按字面保留。

---

## 4. 软分段符（写作辅助）

转写时常「一句一行」，中间还会留空行方便校对；定稿渲染时需要**合成一个 HTML 段落**。  
用行尾反斜杠 `\` 声明「后面仍属同一段」。

### 4.1 语法

```text
soft-join-line = line-content '\' [ spaces ] linebreak
```

- **唯一标记**：行尾单个 `\`（U+005C REVERSE SOLIDUS）  
- 其后仅允许空白，再换行  
- 同一逻辑段的**最后一句不要**加 `\`  
- 行内转义（如 `\.`、`\*`）不变；**仅当 `\` 是该行最后一个非空白字符时**才视为软分段符  

**与 CommonMark 的差异**：CommonMark 行尾 `\` 表示硬换行 `<br>`。JMD **覆盖**该语义——行尾 `\` = 软分段合并，不生成 `<br>`。

### 4.2 合并规则

对每个以软分段符结尾的行：

1. 去掉行尾 `\` 及其后空白  
2. 丢弃紧随其后的**全部空行**  
3. 与下一句「非空内容行」拼进同一段落：  
   - 若接合处两侧均为 CJK 字符（含全角标点）→ **直接拼接，不插入空格**  
   - 否则插入一个 ASCII 空格  
4. 下一句若仍以 `\` 结尾 → 继续 1–3，直到某句不以 `\` 结尾  

**真正分段**（新开 `<p>`）：某句**没有**行尾 `\`，且其后出现空行。

### 4.3 示例

源码（一句一行，中间空行）：

```text
世祖光武皇帝讳秀，字文叔，&[南阳]&[蔡阳]人，\

@[汉高祖刘邦|liu-bang]九世之孙也，\

出自@[汉景帝|han-jing]生长沙定王@[刘发|liu-fa]。
```

渲染逻辑正文（同一段）：

```text
世祖光武皇帝讳秀，字文叔，南阳蔡阳人，汉高祖刘邦九世之孙也，出自汉景帝生长沙定王刘发。
```

（实体仍按 §7 渲染为带链接的标签；上例仅示意空白已消除。）

### 4.4 编辑器

- TextMate / 语义高亮须突出行尾 `\`  
- 预览中**不显示**该反斜杠  

---

## 5. 词法约定

### 5.1 结构 ID / 脚注 label / 修订 id

```text
struct-id = [A-Za-z][A-Za-z0-9_-]*
footnote-label = 1*( ALPHA / DIGIT / '_' / '-' )   ; 不得以 "c:" 开头
rev-id = 1*( ALPHA / DIGIT / '_' / '-' )
```

大小写敏感；单文件内结构锚点不得重复。

### 5.2 实体 ID（消歧键）

```text
entity-id = 1*( ALPHA / DIGIT / '_' / '-' )
```

允许纯数字（如 `1024`、`627`）。大小写敏感。

### 5.3 实体显示名

```text
entity-name = 1*name-char
name-char   = 字母 / 数字 / CJK 统一表意文字及相关标点（如 「」·・〔〕）
            ; 不得包含：空白、[ ] |
```

---

## 6. Frontmatter

### 6.1 语法

仅允许文档最开头（可有 UTF-8 BOM）；TOML 围栏：

```text
+++
title = "…"
+++
```

开/闭围栏行须恰好为 `+++`。

### 6.2 语义

- 成功 → `Document.frontmatter` 为对象  
- TOML 失败 → Error，仍继续解析正文  
- 无 Frontmatter → `{}`  

推荐字段：`title`、`author`（不强制）。

### 6.3 偏移

Frontmatter（含围栏）占用字节 `N` 后：`body_byte_offset = N`。  
正文相对偏移 `k` → 源文档偏移 `body_byte_offset + k`。诊断行号一律相对源文档。

---

## 7. 实体标注（核心，仅括号形）

### 7.1 标记字母

| 前缀 | 类型 `EntityKind` | 含义 | 示例 |
| --- | --- | --- | --- |
| `@` | `Person` | 人名 | `@[唐太宗\|1024]` |
| `&` | `Place` | 地名 | `&[长安]`、`&[洛阳\|901]` |
| `#` | `Time` | 时间 / 年号 | `#[贞观元年\|627]` |
| `$` | `Office` | 官职 | `$[尚书令]`、`$[中书令\|55]` |

### 7.2 唯一合法形式

```text
entity = marker '[' entity-name ( '|' entity-id )? ']'
marker = '@' / '&' / '#' / '$'
```

**`entity-name` = 原文可见用字**（渲染后读者看到的字面），不是知识库全名。  
**`entity-id` = 消歧 / 超链键**（可指向权威库条目）。

| 原文语境 | 源码写法 | 渲染可见 | entity_id |
| --- | --- | --- | --- |
| 「发生舂陵节侯买」 | `@[发\|liu-fa]生$[舂陵节侯]@[买\|liu-mai]` | 发生舂陵节侯买 | `liu-fa` / `liu-mai` |
| 「唐太宗」已是原文 | `@[唐太宗\|1024]` | 唐太宗 | `1024` |
| 仅标注、暂不消歧 | `&[长安]` | 长安 | — |

错误示范：原文只有「发」，却写成 `@[刘发|…]` —— 渲染会变成「刘发」，**破坏原文面貌**。

**非法（本版本不识别为实体）**：裸写 `@李白`、`&长安`、`#贞观元年`、`$尚书令`。  
若出现 `@[` / `&[` / `#[` / `$[` 却缺少闭合 `]` → Error：`未闭合的实体标注`。

### 7.3 与 Markdown 的边界

| 情况 | 规则 |
| --- | --- |
| ATX 标题 | 行首 `#` + **空白** + 标题文本 → 标题 |
| 时间实体 | 仅 `#[…]` |
| GFM 脚注 | `[^label]` |
| 结构锚点 | 仅 `{#struct-id}` |
| 颜色值 | `{color=#ff5500}` 内的 `#` 属染色语法 |
| 数学 | **关闭**；`$` 仅作 `$[…]` 官职前缀 |
| HTML 实体 | `&nbsp;` 等仍按 Markdown；地名必须写成 `&[长安]` |

### 7.4 AST 节点

```text
Entity {
  kind: Person | Place | Time | Office
  name: string
  entity_id: string?
  span: Span
}
```

### 7.5 消歧与超链语义

- 有 `entity_id`：已消歧，可链 `entity://{kind}/{entity_id}`  
- 无 `entity_id`：未消歧；渲染保留 kind + name，供补全  
- 同名不同 ID 允许并存  
- 单文件阶段不对外部库做死链检测（留给 LSP / 实体索引）  
- 可选 Warning：未消歧实体（默认关）

### 7.6 渲染

```html
<a class="jmd-entity jmd-person" data-kind="person" data-id="1024" href="entity://person/1024">唐太宗</a>
<span class="jmd-entity jmd-place" data-kind="place">长安</span>
<a class="jmd-entity jmd-time" data-kind="time" data-id="627" href="entity://time/627">贞观元年</a>
<span class="jmd-entity jmd-office" data-kind="office">尚书令</span>
```

- 有 `entity_id` → `<a href="entity://…">`  
- 无 `entity_id` → `<span>`（须带 `data-kind`）

---

## 8. 结构锚点

用于章节定位；**文档内跳转使用标准 Markdown 链接**，例如 `[见上文](#sec-1)`。

```text
{#struct-id}
```

- 写入结构符号表；重复 → Warning  
- 标题上渲染为元素 `id`  
- **不提供** `[[…]]` Wikilink 语法  

---

## 9. 文本染色

### 9.1 Class

```text
'[' text ']' '{.' class-name '}'
```

例：`[警告文本]{.red}`

### 9.2 Color

```text
'==' text '==' '{color=' color-value '}'
color-value = '#' hex{3/6} | ident
```

例：`==重点内容=={color=#ff5500}`

### 9.3 渲染

```html
<span class="jmd-style red">…</span>
<span class="jmd-style" style="color:#ff5500">…</span>
```

非法颜色 → Warning，保留纯文本。

---

## 10. 侧栏修订（语法预留）

用于日后「正文高亮 + 侧栏修订卡片」审阅模式。**本版本只锁定语法与 AST 占位，不实现侧栏联动、不要求定义闭环。**

### 10.1 正文引用（已定）

贴在原文本之后，形态类似脚注，但 label 固定以 `c:` 开头：

```text
'[^c:' rev-id ']'
```

例：

```text
这段逻辑有待商榷[^c:rev_01]
```

- 解析为 `RevisionRef { id: "rev_01", span }`（`id` **不含** `c:` 前缀）  
- **不得**当作 GFM 脚注处理  

### 10.2 定义行（预留，本版本可不实现语义）

```text
'[^c:' rev-id ']:' [ spaces ] content
```

例：`[^c:rev_01]: 张三: 建议改用异步队列`

- 解析器可识别并收入 `revision_defs`，或暂时当作普通文本/忽略  
- 侧栏卡片、作者字段、与正文对齐等 **后续 SPEC 再补**  

### 10.3 本版本渲染最低要求

- 引用处输出可识别占位，例如：

```html
<sup class="jmd-revision" data-rev="rev_01"><a href="#jmd-rev-rev_01">c:rev_01</a></sup>
```

- 不要求侧栏 `comments` 数组完备；`RenderOutput.revisions` 可为空数组  

---

## 11. 标准脚注（GFM，须可点击跳转）

### 11.1 语法

```text
正文引用：  [^footnote-label]
底部定义：  [^footnote-label]: 出自《资治通鉴》
```

- 遵循 GFM footnotes  
- `footnote-label` **不得**匹配 `c:…`（该形式专属 §10 修订）  
- 例：`正文说明[^1]。` + `[^1]: 出自《资治通鉴》`  

### 11.2 点击跳转（规范性要求）

渲染 HTML **必须**提供双向可点击导航：

| 方向 | 行为 |
| --- | --- |
| 正文引用 → 定义 | 点击引用序号，跳到文末对应脚注定义 |
| 定义 → 正文引用 | 点击定义处返回链，跳回正文引用位置 |

实现约定（与常见 GFM HTML 对齐，id 前缀可配置）：

```html
<!-- 引用 -->
<sup><a href="#user-content-fn-1" id="user-content-fnref-1" data-footnote-ref>1</a></sup>

<!-- 定义（节选） -->
<li id="user-content-fn-1">
  <p>出自《资治通鉴》
    <a href="#user-content-fnref-1" data-footnote-backref>↩</a>
  </p>
</li>
```

预览器 / Webview 须保证页内锚点可滚动定位（同一 `body_html` 文档内）。

### 11.3 闭环诊断

| 条件 | 级别 | 消息 |
| --- | --- | --- |
| 引用无定义 | Error | `缺失脚注定义 [^label]` |
| 定义无引用 | Warning | `未使用的脚注 [^label]` |

---

## 12. 对外 API 契约（`doc-core`）

### 12.1 `parse(source) -> Document`

```text
Document {
  source: string
  frontmatter: object
  body_byte_offset: u32
  entities: [Entity]
  anchors: [{ id, span }]
  styled_spans: [{ text, class?, color?, span }]
  revision_refs: [{ id, span }]       # [^c:rev_01]
  revision_defs: [{ id, content, span }]  # 可空；语义后续补
  footnotes: [{ label, span, kind: Ref | Def }]
  parse_errors: [Diagnostic]
}
```

### 12.2 `validate(doc) -> [Diagnostic]`

重复锚点、**脚注**闭环、未闭合实体等。修订 `[^c:…]` 本版本不做缺失定义 Error。实体外部 ID 不做强制校验。

### 12.3 `render(doc) -> RenderOutput`

```text
RenderOutput {
  frontmatter: object
  body_html: string          # 含可点击脚注双向锚点
  entities: [EntityView]     # 可选：侧栏实体列表
  revisions: [RevisionView]  # 本版本可固定为空
}
```

---

## 13. 诊断级别

| 级别 | 用途 |
| --- | --- |
| Error | Frontmatter 无效、未闭合实体、脚注缺失定义 |
| Warning | 重复结构锚点、未使用脚注、非法 color、（可选）未消歧实体 |
| Info | 预留（含修订标记待侧栏实现） |

---

## 14. 符合性用例

### 14.1 `examples/guangwu.jmd`（光武纪摘录）

| 检查项 | 期望 |
| --- | --- |
| Frontmatter | `title` / `author` 正确 |
| 实体 | Person / Place / Time / Office 均为括号形；显示名与原文用字一致（如 `@[发\|liu-fa]` 渲染为「发」） |
| 软分段 | 源码含行尾 `\` 与空行；`body_html` 对应内容在**同一** `<p>` 内，且不含该反斜杠 |
| 结构锚点 | 含 `guangwu-1`；可用 `[文](#guangwu-1)` 跳转 |
| 无 Wikilink | 源码中不出现 `[[` |
| 修订预留 | 含 `[^c:rev_01]`；不因无定义而 Error |
| 脚注 | `[^1]` 有定义且可点击跳转 |
| 染色 | 按 §9 |
| `validate` | 无 Error |

### 14.2 `examples/broken-links.jmd`

| 检查项 | 期望 |
| --- | --- |
| 重复 `{#dup-id}` | Warning |
| 未闭合实体 `@[残缺` | Error |
| 脚注引用无定义 `[^99]` | Error |
| 修订 `[^c:orphan]` | **不**因此报 Error（预留语法） |

---

## 15. 版本与变更

| 版本 | 日期 | 说明 |
| --- | --- | --- |
| 0.1 | 2026-11-25 | 首版五类扩展 |
| 0.2 | 2026-11-25 | 历史实体核心；评论 `[^~id]` |
| 0.3 | 2026-11-25 | 仅括号形实体；移除 Wikilink |
| 0.4 | 2026-11-25 | 修订 `[^c:id]` 预留；脚注双向跳转 |
| 0.5 | 2026-11-25 | 软分段符；符合性示例改为 `guangwu.jmd`；标记定为行尾 `\` |

---

## 附录 A. 最小示意（光武纪 + 软分段）

```jmd
+++
title = "后汉书 · 光武帝纪（摘录）"
+++

# 世祖本纪 {#guangwu-1}

世祖@[光武|liu-xiu]皇帝讳@[秀|liu-xiu]，字文叔，&[南阳]&[蔡阳]人，\

@[发|liu-fa]生$[舂陵节侯]@[买|liu-mai]。\

@[王莽|wang-mang]#[天凤]中，乃之&[长安]。[^1]
形貌存疑[^c:rev_01]。

[^1]: 出自《后汉书·光武帝纪》。
```
