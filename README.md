# butterfly-dream

JMD 文档语言：Rust 核心解析器 + LSP + WASM 预览 + VS Code 扩展（Monorepo）。

语言规范见 [SPEC.md](SPEC.md)（实现与示例以 SPEC 为准）。

## 结构

```text
crates/doc-core   # 解析 / 校验 / 渲染（无 I/O，可编 WASM）
crates/doc-lsp    # Language Server（tower-lsp）
crates/doc-wasm   # WebAssembly 胶水层
editors/vscode    # VS Code 扩展 + webview
examples/         # .jmd 示例文档
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

# 编译扩展并用 Cursor 开发宿主打开 examples/
pnpm run dev:vscode

# 把 examples/*.jmd 编译成同名 .html（如 guangwu.jmd → guangwu.html）
pnpm run dev:html
```


## 文件类型

- 语言 ID：`jmd`
- 扩展名：`.jmd`
