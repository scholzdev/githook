use crate::document::DocumentState;
use tower_lsp::lsp_types::*;

pub fn get_document_symbols(_doc: &DocumentState) -> Vec<DocumentSymbol> {
    let mut symbols = Vec::new();

    let macros = _doc
        .ast
        .as_ref()
        .map(|ast| crate::ast_utils::extract_macros(ast))
        .unwrap_or_default();

    for macro_info in &macros {
        #[allow(deprecated)]
        let symbol = DocumentSymbol {
            name: macro_info.name.clone(),
            detail: Some(format!("macro({})", macro_info.params.join(", "))),
            kind: SymbolKind::FUNCTION,
            tags: None,
            deprecated: None,
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: macro_info.name.len() as u32,
                },
            },
            selection_range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: macro_info.name.len() as u32,
                },
            },
            children: None,
        };
        symbols.push(symbol);
    }

    for (alias, path) in &[] as &[(String, String)] {
        let detail = format!("import \"{}\"", path);
        #[allow(deprecated)]
        let symbol = DocumentSymbol {
            name: alias.clone(),
            detail: Some(detail),
            kind: SymbolKind::MODULE,
            tags: None,
            deprecated: None,
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 0,
                },
            },
            selection_range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 0,
                },
            },
            children: None,
        };
        symbols.push(symbol);
    }

    symbols
}
