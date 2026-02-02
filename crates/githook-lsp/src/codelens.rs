use crate::document::DocumentState;
use std::collections::HashMap;
use tower_lsp::lsp_types::*;

pub fn get_code_lens(
    doc: &DocumentState,
    all_documents: &HashMap<String, DocumentState>,
    current_uri: &str,
) -> Vec<CodeLens> {
    let mut lenses = Vec::new();

    let macros = doc
        .ast
        .as_ref()
        .map(|ast| crate::ast_utils::extract_macros(ast))
        .unwrap_or_default();

    for macro_info in &macros {
        let local_count = doc.text.matches(&format!("@{}", macro_info.name)).count();

        let mut cross_file_count = 0;

        for (other_uri, other_doc) in all_documents.iter() {
            if other_uri == current_uri {
                continue;
            }

            for (namespace, import_uri) in &[] as &[(String, String)] {
                if import_uri == current_uri {
                    let pattern = format!("@{}:{}", namespace, macro_info.name);
                    cross_file_count += other_doc.text.matches(&pattern).count();
                }
            }
        }

        let ref_count = local_count + cross_file_count;

        lenses.push(CodeLens {
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
            command: None,
            data: Some(serde_json::json!({
                "refCount": ref_count,
                "macroName": macro_info.name,
            })),
        });
    }

    lenses
}
