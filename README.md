# butterfly-dream

JMD 文档语言：Rust 核心解析器 + LSP + WASM 预览 + VS Code 扩展（Monorepo）。

语言规范见 [SPEC.md](SPEC.md)（实现与示例以 SPEC 为准）。

## 结构

```text
crates/doc-core            # 解析 / 校验 / 渲染（无 I/O，可编 WASM）
crates/doc-core/examples/  # .rymd 示例 + preview 模板 / 渲染入口
crates/doc-lsp             # Language Server（tower-lsp）
crates/doc-wasm            # WebAssembly 胶水层
editors/vscode             # VS Code 扩展 + webview
```

## 要求

- Rust stable（见 `rust-toolchain.toml`，edition 2024）
- Node.js（编译 VS Code 扩展时）

## 常用命令

```bash
cargo test -p doc-core
cargo build -p doc-lsp
cargo check -p doc-wasm

# 一键构建（需 wasm-pack）
pnpm run build:all

# 编译 LSP+扩展，并用 Extension Development Host 打开本仓库（*.rymd → 语言 RYMD）
pnpm run dev:vscode
# 在新窗口里打开 crates/doc-core/examples/*.rymd；状态栏应显示 RYMD，不是 Plain Text

# 把 crates/doc-core/examples/*.rymd 编译成同名 .html
pnpm run dev:html
```

## 文件类型

- 语言 ID：`rymd`
- 扩展名：`.rymd`
