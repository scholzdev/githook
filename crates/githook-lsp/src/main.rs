use tracing::info;
use tower_lsp::{LspService, Server};

mod backend;
mod diagnostics;
mod completion;
mod document;
mod docs;
mod goto_definition;
mod hover;
mod inlay_hints;
mod import_resolver;
mod symbols;
mod references;
mod rename;
mod folding;
mod codelens;
mod semantic_tokens;
mod documentlinks;
mod ast_utils;

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
    
    Server::new(stdin, stdout, socket)
        .serve(service)
        .await;

    info!("Githook Language Server stopped");
}
