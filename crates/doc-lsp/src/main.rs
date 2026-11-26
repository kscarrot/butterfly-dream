mod diagnostics;
mod folding;
mod hover;
mod navigation;
mod presentation;
mod semantic;
mod server;

use tower_lsp::{LspService, Server};

use crate::server::Backend;

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
