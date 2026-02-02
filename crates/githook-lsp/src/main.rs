use tower_lsp::{LspService, Server};
use tracing::info;

mod ast_utils;
mod backend;
mod codelens;
mod completion;
mod diagnostics;
mod docs;
mod document;
mod documentlinks;
mod folding;
mod goto_definition;
mod hover;
mod import_resolver;
mod inlay_hints;
mod references;
mod rename;
mod semantic_tokens;
mod symbols;

use backend::GithookLanguageServer;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_writer(std::io::stderr)
        .init();

    info!("Starting Githook Language Server");

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(GithookLanguageServer::new);

    Server::new(stdin, stdout, socket).serve(service).await;

    info!("Githook Language Server stopped");
}
