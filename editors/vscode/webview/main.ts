/**
 * Webview 入口：调用 doc-wasm 的 `render_preview`，并对齐侧栏。
 * 占位 — WASM pkg 尚未接入。
 */

const bodyEl = document.getElementById("body");
const commentsEl = document.getElementById("comments");

if (bodyEl) {
  bodyEl.innerHTML = "<p>Preview webview stub.</p>";
}

if (commentsEl) {
  commentsEl.innerHTML = "<p>Comments sidebar stub.</p>";
}
