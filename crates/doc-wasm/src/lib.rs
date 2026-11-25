//! WASM 胶水层：把 `doc-core` 的 render 暴露给 VS Code Webview / 浏览器。

use wasm_bindgen::prelude::*;

/// 解析并渲染源文本，返回 JS 侧的 `RenderOutput`。
#[wasm_bindgen]
pub fn render_preview(text: &str) -> Result<JsValue, JsValue> {
    let doc = doc_core::parse(text);
    let output = doc_core::render(&doc);
    serde_wasm_bindgen::to_value(&output).map_err(|e| JsValue::from_str(&e.to_string()))
}
