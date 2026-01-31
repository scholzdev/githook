use tower_lsp::lsp_types::*;
use tower_lsp::Client;
use githook_syntax::error::Span;

/// Convert our ParseError to LSP Diagnostic
pub fn parse_error_to_diagnostic(error: &str, span: &Option<Span>, _source: &str) -> Diagnostic {
    let range = if let Some(span) = span {
        span_to_range(*span)
    } else {
        // If no span available, point to start of file
        Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: 0, character: 0 },
        }
    };

    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::ERROR),
        code: None,
        code_description: None,
        source: Some("githook".to_string()),
        message: error.to_string(),
        related_information: None,
        tags: None,
        data: None,
    }
}

/// Convert our Span to LSP Range
pub fn span_to_range(span: Span) -> Range {
    Range {
        start: Position {
            line: span.line.saturating_sub(1) as u32, // Span is 1-indexed, LSP is 0-indexed
            character: span.col.saturating_sub(1) as u32,
        },
        end: Position {
            line: span.line.saturating_sub(1) as u32,
            character: (span.col + (span.end - span.start)).saturating_sub(1) as u32,
        },
    }
}

/// Publish diagnostics to the client
pub async fn publish_diagnostics(client: &Client, uri: Url, diagnostics: Vec<Diagnostic>) {
    client
        .publish_diagnostics(uri, diagnostics, None)
        .await;
}
