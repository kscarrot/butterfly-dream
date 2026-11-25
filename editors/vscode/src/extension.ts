import * as vscode from "vscode";
import { PreviewManager } from "./previewManager";

export function activate(context: vscode.ExtensionContext): void {
  const previewManager = new PreviewManager(context);

  context.subscriptions.push(
    vscode.commands.registerCommand("butterflyDream.openPreview", () => {
      previewManager.openPreview();
    }),
  );
}

export function deactivate(): void {}
