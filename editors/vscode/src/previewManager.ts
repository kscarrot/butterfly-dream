import * as vscode from "vscode";

/** 管理 jmd 预览 Webview 面板（占位实现）。 */
export class PreviewManager {
  private panel: vscode.WebviewPanel | undefined;

  constructor(private readonly context: vscode.ExtensionContext) {}

  openPreview(): void {
    if (this.panel) {
      this.panel.reveal(vscode.ViewColumn.Beside);
      return;
    }

    this.panel = vscode.window.createWebviewPanel(
      "butterflyDreamPreview",
      "JMD Preview",
      vscode.ViewColumn.Beside,
      {
        enableScripts: true,
        retainContextWhenHidden: true,
      },
    );

    this.panel.webview.html = this.getPlaceholderHtml();
    this.panel.onDidDispose(() => {
      this.panel = undefined;
    });
  }

  private getPlaceholderHtml(): string {
    return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>JMD Preview</title>
  <style>
    body { font-family: var(--vscode-font-family); padding: 1rem; color: var(--vscode-foreground); }
  </style>
</head>
<body>
  <h1>JMD Preview</h1>
  <p>Preview panel stub — WASM render will be wired later.</p>
</body>
</html>`;
  }
}
