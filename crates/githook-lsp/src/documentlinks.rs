use tower_lsp::lsp_types::*;
use crate::document::DocumentState;
use crate::import_resolver::resolve_import_path;

pub fn get_document_links(doc: &DocumentState, current_uri: &str) -> Vec<DocumentLink> {
    let mut links = Vec::new();
    
    let imports = doc.ast.as_ref()
        .map(|ast| crate::ast_utils::extract_imports(ast))
        .unwrap_or_default();
    
    for import_info in &imports {
        let import_text = format!("\"{}\"", import_info.path);
        
        for (line_num, line) in doc.text.lines().enumerate() {
            if let Some(pos) = line.find(&import_text) {
                if let Some(resolved) = resolve_import_path(current_uri, &import_info.path) {
                    let file_uri = format!("file://{}", resolved.to_string_lossy());
                    
                    if let Ok(uri) = Url::parse(&file_uri) {
                        links.push(DocumentLink {
                            range: Range {
                                start: Position {
                                    line: line_num as u32,
                                    character: (pos + 1) as u32,
                                },
                                end: Position {
                                    line: line_num as u32,
                                    character: (pos + import_info.path.len() + 1) as u32,
                                },
                            },
                            target: Some(uri),
                            tooltip: Some(format!("Open {}", import_info.path)),
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
