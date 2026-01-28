use tower_lsp::lsp_types::*;
use crate::document::DocumentState;
use std::collections::HashMap;

/// Get code lens (reference counts) for symbols
pub fn get_code_lens(doc: &DocumentState, all_documents: &HashMap<String, DocumentState>, current_uri: &str) -> Vec<CodeLens> {
    let mut lenses = Vec::new();
    
    for (name, span, _body) in &doc.macro_definitions {
        // Count local references in current document
        let local_count = doc.text.matches(&format!("@{}", name)).count();
        
        // Count cross-file references from documents that import this file
        let mut cross_file_count = 0;
        
        // Extract namespace from imports that point to current file
        for (other_uri, other_doc) in all_documents.iter() {
            if other_uri == current_uri {
                continue; // Skip current document
            }
            
            // Check if this document imports the current file
            for (namespace, import_uri) in &other_doc.imports {
                if import_uri == current_uri {
                    // Count namespaced references like @namespace:macro_name
                    let pattern = format!("@{}:{}", namespace, name);
                    cross_file_count += other_doc.text.matches(&pattern).count();
                }
            }
        }
        
        let ref_count = local_count + cross_file_count;
        
        lenses.push(CodeLens {
            range: Range {
                start: Position {
                    line: (span.line - 1) as u32,
                    character: 0,
                },
                end: Position {
                    line: (span.line - 1) as u32,
                    character: 0,
                },
            },
            command: None, // No command - just display reference count
            data: Some(serde_json::json!({
                "refCount": ref_count,
                "macroName": name,
            })),
        });
    }
    
    lenses
}
