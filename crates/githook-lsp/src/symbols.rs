use crate::document::DocumentState;
use tower_lsp::lsp_types::*;

pub fn get_document_symbols(doc: &DocumentState) -> Vec<DocumentSymbol> {
    let mut symbols = Vec::new();

    let macros = doc
        .ast
        .as_ref()
        .map(|ast| crate::ast_utils::extract_macros(ast))
        .unwrap_or_default();

    for macro_info in &macros {
        let start_line = (macro_info.span.line.saturating_sub(1)) as u32;
        let start_char = (macro_info.span.col.saturating_sub(1)) as u32;

        #[allow(deprecated)]
        let symbol = DocumentSymbol {
            name: macro_info.name.clone(),
            detail: Some(format!("macro({})", macro_info.params.join(", "))),
            kind: SymbolKind::FUNCTION,
            tags: None,
            deprecated: None,
            range: Range {
                start: Position {
                    line: start_line,
                    character: start_char,
                },
                end: Position {
                    line: start_line,
                    character: start_char + macro_info.name.len() as u32 + 6,
                },
            },
            selection_range: Range {
                start: Position {
                    line: start_line,
                    character: start_char + 6, // after "macro "
                },
                end: Position {
                    line: start_line,
                    character: start_char + 6 + macro_info.name.len() as u32,
                },
            },
            children: None,
        };
        symbols.push(symbol);
    }

    let imports = doc
        .ast
        .as_ref()
        .map(|ast| crate::ast_utils::extract_imports(ast))
        .unwrap_or_default();

    for import_info in &imports {
        if let Some(alias) = &import_info.alias {
            let detail = format!("import \"{}\"", import_info.path);
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
    }

    symbols
}
