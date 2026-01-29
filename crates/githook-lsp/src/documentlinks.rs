use tower_lsp::lsp_types::*;
use crate::document::DocumentState;
use crate::import_resolver::resolve_import_path;

/// Get document links for import statements
pub fn get_document_links(doc: &DocumentState, current_uri: &str) -> Vec<DocumentLink> {
    let mut links = Vec::new();
    
    // TODO: Extract imports from AST
    for (_, import_path) in &[] as &[(String, String)] {
        // Find the import statement in the text
        let import_text = format!("\"{}\"", import_path);
        
        for (line_num, line) in doc.text.lines().enumerate() {
            if let Some(pos) = line.find(&import_text) {
                // Resolve the import path to absolute path
                if let Some(resolved) = resolve_import_path(current_uri, import_path) {
                    let file_uri = format!("file://{}", resolved.to_string_lossy());
                    
                    if let Ok(uri) = Url::parse(&file_uri) {
                        links.push(DocumentLink {
                            range: Range {
                                start: Position {
                                    line: line_num as u32,
                                    character: (pos + 1) as u32, // Skip opening quote
                                },
                                end: Position {
                                    line: line_num as u32,
                                    character: (pos + import_path.len() + 1) as u32,
                                },
                            },
                            target: Some(uri),
                            tooltip: Some(format!("Open {}", import_path)),
                            data: None,
                        });
                    }
                }
                break;
            }
        }
    }
    
    links
}
