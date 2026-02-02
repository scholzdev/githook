use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use tracing::info;

use crate::codelens::get_code_lens;
use crate::completion::get_completions;
use crate::diagnostics::publish_diagnostics;
use crate::document::DocumentState;
use crate::documentlinks::get_document_links;
use crate::folding::get_folding_ranges;
use crate::goto_definition::get_definition;
use crate::hover::get_hover;
use crate::inlay_hints::get_inlay_hints;
use crate::references::find_references;
use crate::rename::{execute_rename, prepare_rename};
use crate::semantic_tokens::{get_legend, get_semantic_tokens};
use crate::symbols::get_document_symbols;

pub struct GithookLanguageServer {
    client: Client,
    documents: Arc<RwLock<HashMap<String, DocumentState>>>,
}

impl GithookLanguageServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn update_document(&self, uri: Url, text: String) {
        let state = DocumentState::new(text.clone(), Some(uri.as_ref()));

        let diagnostics = state.diagnostics().unwrap_or_default();
        publish_diagnostics(&self.client, uri.clone(), diagnostics).await;

        let mut documents = self.documents.write().await;
        documents.insert(uri.to_string(), state);
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for GithookLanguageServer {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        info!("Initializing Githook Language Server");

        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "githook-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        ..Default::default()
                    },
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![
                        "@".to_string(),
                        ".".to_string(),
                        " ".to_string(),
                    ]),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                })),
                folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
                code_lens_provider: Some(CodeLensOptions {
                    resolve_provider: Some(true),
                }),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: get_legend(),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            range: Some(false),
                            work_done_progress_options: WorkDoneProgressOptions::default(),
                        },
                    ),
                ),
                document_link_provider: Some(DocumentLinkOptions {
                    resolve_provider: Some(false),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                }),
                inlay_hint_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        info!("Githook Language Server initialized");
        self.client
            .log_message(MessageType::INFO, "Githook LSP ready")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        info!("Shutting down Githook Language Server");
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        info!("Document opened: {}", params.text_document.uri);
        self.update_document(params.text_document.uri, params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        info!("Document changed: {}", params.text_document.uri);
        if let Some(change) = params.content_changes.into_iter().next() {
            self.update_document(params.text_document.uri, change.text)
                .await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        info!("Document closed: {}", params.text_document.uri);
        let mut documents = self.documents.write().await;
        documents.remove(&params.text_document.uri.to_string());
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let documents = self.documents.read().await;
        let uri = params.text_document_position.text_document.uri.to_string();

        if let Some(doc) = documents.get(&uri) {
            let position = params.text_document_position.position;
            let completions = get_completions(doc, position);
            return Ok(Some(CompletionResponse::Array(completions)));
        }

        Ok(None)
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let documents = self.documents.read().await;
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();

        if let Some(doc) = documents.get(&uri) {
            let position = params.text_document_position_params.position;
            return Ok(get_hover(doc, position, &uri));
        }

        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let position = params.text_document_position_params.position;

        let documents = self.documents.read().await;
        if let Some(doc) = documents.get(uri.as_str())
            && let Some(location) = get_definition(doc, position, uri.as_str())
        {
            return Ok(Some(GotoDefinitionResponse::Scalar(location)));
        }

        Ok(None)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri.to_string();
        let documents = self.documents.read().await;

        if let Some(doc) = documents.get(&uri) {
            let symbols = get_document_symbols(doc);
            return Ok(Some(DocumentSymbolResponse::Nested(symbols)));
        }

        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri.clone();
        let position = params.text_document_position.position;
        let include_declaration = params.context.include_declaration;

        let documents = self.documents.read().await;
        if let Some(doc) = documents.get(uri.as_str()) {
            let mut refs = find_references(doc, position, include_declaration);
            for loc in &mut refs {
                loc.uri = uri.clone();
            }
            return Ok(Some(refs));
        }

        Ok(None)
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = params.text_document.uri.to_string();
        let position = params.position;

        let documents = self.documents.read().await;
        if let Some(doc) = documents.get(&uri)
            && let Some(range) = prepare_rename(doc, position)
        {
            return Ok(Some(PrepareRenameResponse::Range(range)));
        }

        Ok(None)
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri.to_string();
        let position = params.text_document_position.position;
        let new_name = params.new_name;

        let documents = self.documents.read().await;
        if let Some(doc) = documents.get(&uri) {
            return Ok(execute_rename(doc, position, new_name));
        }

        Ok(None)
    }

    async fn folding_range(&self, params: FoldingRangeParams) -> Result<Option<Vec<FoldingRange>>> {
        let uri = params.text_document.uri.to_string();
        let documents = self.documents.read().await;

        if let Some(doc) = documents.get(&uri) {
            let ranges = get_folding_ranges(&doc.ast);
            return Ok(Some(ranges));
        }

        Ok(None)
    }

    async fn code_lens(&self, params: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
        let uri = params.text_document.uri.to_string();
        let documents = self.documents.read().await;

        if let Some(doc) = documents.get(&uri) {
            let lenses = get_code_lens(doc, &documents, &uri);
            return Ok(Some(lenses));
        }

        Ok(None)
    }

    async fn code_lens_resolve(&self, mut code_lens: CodeLens) -> Result<CodeLens> {
        if let Some(data) = &code_lens.data
            && let Ok(info) = serde_json::from_value::<serde_json::Value>(data.clone())
            && let (Some(count), Some(_name)) = (
                info.get("refCount").and_then(|v| v.as_u64()),
                info.get("macroName").and_then(|v| v.as_str()),
            )
        {
            let message = if count == 1 {
                "1 reference".to_string()
            } else {
                format!("{} references", count)
            };

            code_lens.command = Some(Command {
                title: message,
                command: "".to_string(),
                arguments: None,
            });
        }

        Ok(code_lens)
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri.to_string();
        let documents = self.documents.read().await;

        if let Some(doc) = documents.get(&uri) {
            let tokens = get_semantic_tokens(&doc.ast, &doc.text);
            return Ok(Some(SemanticTokensResult::Tokens(tokens)));
        }

        Ok(None)
    }

    async fn document_link(&self, params: DocumentLinkParams) -> Result<Option<Vec<DocumentLink>>> {
        let uri = params.text_document.uri.to_string();
        let documents = self.documents.read().await;

        if let Some(doc) = documents.get(&uri) {
            let links = get_document_links(doc, &uri);
            return Ok(Some(links));
        }

        Ok(None)
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        let uri = params.text_document.uri.to_string();
        let documents = self.documents.read().await;

        if let Some(doc) = documents.get(&uri) {
            let hints = get_inlay_hints(doc, params.range);
            return Ok(Some(hints));
        }

        Ok(None)
    }
}
