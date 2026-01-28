use tracing::info;
use tower_lsp::{LspService, Server};

mod backend;
mod diagnostics;
mod completion;
mod document;
mod goto_definition;
mod hover;
mod import_resolver;
mod symbols;
mod references;
mod rename;
mod folding;
mod codelens;
mod semantic_tokens;
mod documentlinks;

use backend::GithookLanguageServer;

#[tokio::main]
async fn main() {
    // Initialize tracing for logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_writer(std::io::stderr)
        .init();

    info!("Starting Githook Language Server");

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| GithookLanguageServer::new(client));
    
    Server::new(stdin, stdout, socket)
        .serve(service)
        .await;

    info!("Githook Language Server stopped");
}
