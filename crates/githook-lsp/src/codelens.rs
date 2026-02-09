use crate::document::DocumentState;
use std::collections::HashMap;
use tower_lsp::lsp_types::*;

pub fn get_code_lens(
    doc: &DocumentState,
    _all_documents: &HashMap<String, DocumentState>,
    _current_uri: &str,
) -> Vec<CodeLens> {
    let mut lenses = Vec::new();

    let macros = doc
        .ast
        .as_ref()
        .map(|ast| crate::ast_utils::extract_macros(ast))
        .unwrap_or_default();

    for macro_info in &macros {
        let local_count = doc.text.matches(&format!("@{}", macro_info.name)).count();

        let ref_count = local_count;

        let start_line = (macro_info.span.line.saturating_sub(1)) as u32;
        let start_char = (macro_info.span.col.saturating_sub(1)) as u32;

        lenses.push(CodeLens {
            range: Range {
                start: Position {
                    line: start_line,
                    character: start_char,
                },
                end: Position {
                    line: start_line,
                    character: start_char + macro_info.name.len() as u32 + 6, // "macro " prefix
                },
            },
            command: None,
            data: Some(serde_json::json!({
                "refCount": ref_count,
                "macroName": macro_info.name,
            })),
        });
    }

    lenses
}
