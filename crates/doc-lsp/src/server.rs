use std::collections::HashMap;
use std::sync::RwLock;

use doc_core::{parse_editor, validate, Document};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::diagnostics;
use crate::folding;
use crate::hover;
use crate::navigation;
use crate::presentation;
use crate::semantic;

#[derive(Debug)]
struct DocState {
    doc: Document,
}

#[derive(Debug)]
pub struct Backend {
    client: Client,
    docs: RwLock<HashMap<Url, DocState>>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            docs: RwLock::new(HashMap::new()),
        }
    }

    async fn upsert(&self, uri: Url, source: String) {
        let doc = parse_editor(&source);
        let diags = validate(&doc);
        let lsp_diags = diagnostics::to_lsp(&source, &diags);
        let present = presentation::build(&uri, &doc);

        {
            let mut guard = self.docs.write().unwrap();
            guard.insert(uri.clone(), DocState { doc });
        }

        self.client
            .publish_diagnostics(uri.clone(), lsp_diags, None)
            .await;

        self.client
            .send_notification::<EntityPresentationNotification>(present)
            .await;
    }

    fn with_doc<R>(&self, uri: &Url, f: impl FnOnce(&DocState) -> R) -> Option<R> {
        let guard = self.docs.read().unwrap();
        guard.get(uri).map(f)
    }
}

#[derive(Debug)]
struct EntityPresentationNotification;

impl tower_lsp::lsp_types::notification::Notification for EntityPresentationNotification {
    type Params = presentation::EntityPresentationParams;
    const METHOD: &'static str = presentation::METHOD;
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: semantic::legend(),
                            range: Some(false),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            work_done_progress_options: Default::default(),
                        },
                    ),
                ),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "doc-lsp".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "doc-lsp initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.upsert(params.text_document.uri, params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last() {
            self.upsert(params.text_document.uri, change.text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.docs.write().unwrap().remove(&params.text_document.uri);
        self.client
            .publish_diagnostics(params.text_document.uri, Vec::new(), None)
            .await;
    }

    async fn folding_range(&self, params: FoldingRangeParams) -> Result<Option<Vec<FoldingRange>>> {
        Ok(Some(
            self.with_doc(&params.text_document.uri, |s| folding::folding_ranges(&s.doc))
                .unwrap_or_default(),
        ))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        Ok(self
            .with_doc(&uri, |s| navigation::goto_definition(&s.doc, &uri, pos))
            .flatten()
            .map(GotoDefinitionResponse::Scalar))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        Ok(self.with_doc(&params.text_document.uri, |s| {
            SemanticTokensResult::Tokens(semantic::semantic_tokens(&s.doc))
        }))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        Ok(self.with_doc(&uri, |s| hover::hover(&s.doc, pos)).flatten())
    }
}
