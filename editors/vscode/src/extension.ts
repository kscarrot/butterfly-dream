import * as path from "path";
import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";
import {
  EntityPresentationManager,
  EntityPresentationParams,
} from "./entityPresentation";
import { PreviewManager } from "./previewManager";

let client: LanguageClient | undefined;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  const previewManager = new PreviewManager(context);
  const entityUi = new EntityPresentationManager();

  context.subscriptions.push(
    vscode.commands.registerCommand("butterflyDream.openPreview", () => {
      previewManager.openPreview();
    }),
    { dispose: () => entityUi.dispose() },
  );

  const serverModule = context.asAbsolutePath(path.join("bin", "doc-lsp"));
  const serverOptions: ServerOptions = {
    run: { command: serverModule, transport: TransportKind.stdio },
    debug: { command: serverModule, transport: TransportKind.stdio },
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: "file", language: "rymd" },
      { scheme: "untitled", language: "rymd" },
    ],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher("**/*.rymd"),
    },
  };

  try {
    client = new LanguageClient("doc-lsp", "Butterfly Dream RYMD", serverOptions, clientOptions);
    client.onNotification("jmd/entityPresentation", (params: EntityPresentationParams) => {
      // languageclient 有时把 Uri 解成对象；统一成 string
      const uri =
        typeof params.uri === "string"
          ? params.uri
          : (params.uri as unknown as { toString?: () => string })?.toString?.() ??
            String(params.uri);
      entityUi.apply({ ...params, uri });
    });
    await client.start();
    context.subscriptions.push({ dispose: () => client?.stop() });
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    void vscode.window.showErrorMessage(`Butterfly Dream LSP failed to start: ${msg}`);
    console.error("doc-lsp start failed", err);
  }
}

export async function deactivate(): Promise<void> {
  if (client) {
    await client.stop();
    client = undefined;
  }
}
