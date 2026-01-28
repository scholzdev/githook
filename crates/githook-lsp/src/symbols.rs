use tower_lsp::lsp_types::*;
use crate::document::DocumentState;

/// Get document symbols for outline view
pub fn get_document_symbols(doc: &DocumentState) -> Vec<DocumentSymbol> {
    let mut symbols = Vec::new();
    
    // Add macro definitions
    for (name, span, _body) in &doc.macro_definitions {
        #[allow(deprecated)]
        let symbol = DocumentSymbol {
            name: name.clone(),
            detail: Some("macro".to_string()),
            kind: SymbolKind::FUNCTION,
            tags: None,
            deprecated: None,
            range: Range {
                start: Position {
                    line: (span.line - 1) as u32,
                    character: (span.col - 1) as u32,
                },
                end: Position {
                    line: (span.line - 1) as u32,
                    character: (span.col + name.len()) as u32,
                },
            },
            selection_range: Range {
                start: Position {
                    line: (span.line - 1) as u32,
                    character: (span.col - 1) as u32,
                },
                end: Position {
                    line: (span.line - 1) as u32,
                    character: (span.col + name.len()) as u32,
                },
            },
            children: None,
        };
        symbols.push(symbol);
    }
    
    // Add imports
    for (alias, path) in &doc.imports {
        let detail = format!("import \"{}\"", path);
        #[allow(deprecated)]
        let symbol = DocumentSymbol {
            name: alias.clone(),
            detail: Some(detail),
            kind: SymbolKind::MODULE,
            tags: None,
            deprecated: None,
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 0 },
            },
            selection_range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 0 },
            },
            children: None,
        };
        symbols.push(symbol);
    }
    
    symbols
}
