import * as vscode from "vscode";

interface LspPosition {
  line: number;
  character: number;
}

interface LspRange {
  start: LspPosition;
  end: LspPosition;
}

export interface EntityPresentationParams {
  uri: string;
  entities: {
    kind: string;
    nameRange: LspRange;
    hideRanges: LspRange[];
  }[];
}

/** 隐藏 `@[` / `|id]` / 闭合 `]`；实体名着色交给 LSP semantic tokens。 */
export class EntityPresentationManager {
  private readonly hideType: vscode.TextEditorDecorationType;
  private readonly byUri = new Map<string, vscode.Range[]>();
  private readonly disposables: vscode.Disposable[] = [];

  constructor() {
    // font-size:0 把语法段收成不可见；letter-spacing 尽量吃掉占位
    this.hideType = vscode.window.createTextEditorDecorationType({
      textDecoration: "none; font-size: 0; letter-spacing: -1ch;",
      rangeBehavior: vscode.DecorationRangeBehavior.ClosedClosed,
    });

    this.disposables.push(
      vscode.window.onDidChangeVisibleTextEditors(() => this.repaintAll()),
      vscode.workspace.onDidCloseTextDocument((doc) => {
        this.byUri.delete(keyOf(doc.uri));
      }),
    );
  }

  dispose(): void {
    for (const d of this.disposables) {
      d.dispose();
    }
    this.hideType.dispose();
    this.byUri.clear();
  }

  apply(params: EntityPresentationParams): void {
    const uri = vscode.Uri.parse(params.uri);
    const key = keyOf(uri);
    const hide: vscode.Range[] = [];
    for (const e of params.entities) {
      for (const h of e.hideRanges) {
        hide.push(toRange(h));
      }
    }
    this.byUri.set(key, hide);
    this.paintUri(uri, hide);
  }

  private repaintAll(): void {
    for (const editor of vscode.window.visibleTextEditors) {
      const hide = this.byUri.get(keyOf(editor.document.uri));
      if (hide) {
        editor.setDecorations(this.hideType, hide);
      }
    }
  }

  private paintUri(uri: vscode.Uri, hide: vscode.Range[]): void {
    const key = keyOf(uri);
    for (const editor of vscode.window.visibleTextEditors) {
      if (keyOf(editor.document.uri) !== key) {
        continue;
      }
      editor.setDecorations(this.hideType, hide);
    }
  }
}

function keyOf(uri: vscode.Uri): string {
  return uri.scheme === "file" ? uri.fsPath : uri.toString();
}

function toRange(r: LspRange): vscode.Range {
  return new vscode.Range(r.start.line, r.start.character, r.end.line, r.end.character);
}
