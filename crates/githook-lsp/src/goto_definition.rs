use tower_lsp::lsp_types::*;
use crate::document::DocumentState;
use crate::import_resolver::{resolve_import_path, load_imported_macros, path_to_uri};
use tracing::info;

/// Find the definition location for a symbol at the given position
pub fn get_definition(doc: &DocumentState, position: Position, current_uri: &str) -> Option<Location> {
    let line_idx = position.line as usize;
    let lines: Vec<&str> = doc.text.lines().collect();
    
    if line_idx >= lines.len() {
        return None;
    }
    
    let line = lines[line_idx];
    let char_idx = position.character as usize;
    
    if char_idx == 0 || char_idx > line.len() {
        return None;
    }
    
    let before_cursor = &line[..char_idx];
    
    // Find the start of the current word
    let word_start = before_cursor.rfind(|c: char| c.is_whitespace() || c == '{' || c == '}')
        .map(|pos| pos + 1)
        .unwrap_or(0);
    
    let after_cursor = &line[char_idx..];
    let word_end = char_idx + after_cursor.find(|c: char| c.is_whitespace() || c == '{' || c == '}')
        .unwrap_or(after_cursor.len());
    
    let word = &line[word_start..word_end];
    
    // Check if it's a macro call (starts with @)
    if word.starts_with('@') {
        let macro_ref = &word[1..]; // Remove @
        
        // Check if it's a namespaced macro call (namespace:macro_name)
        if let Some(colon_pos) = macro_ref.find(':') {
            let namespace = &macro_ref[..colon_pos];
            let macro_name = &macro_ref[colon_pos + 1..];
            
            info!("Looking for namespaced macro: {}:{}", namespace, macro_name);
            info!("Available imports: {:?}", doc.imports);
            
            // Find the import with this alias
            for (alias, import_path) in &doc.imports {
                if alias == namespace {
                    info!("Found import: {} -> {}", alias, import_path);
                    
                    // Resolve the imported file path
                    if let Some(resolved_path) = resolve_import_path(current_uri, import_path) {
                        info!("Resolved path: {:?}", resolved_path);
                        
                        // Load and parse the imported file
                        if let Ok(imported_macros) = load_imported_macros(&resolved_path) {
                            info!("Loaded {} macros", imported_macros.len());
                            
                            // Find the macro in the imported file
                            for (name, span, _body) in imported_macros {
                                if name == macro_name {
                                    info!("Found macro {} at line {}", name, span.line);
                                    
                                    let start = Position {
                                        line: (span.line - 1) as u32,
                                        character: (span.col - 1) as u32,
                                    };
                                    let end = Position {
                                        line: (span.line - 1) as u32,
                                        character: (span.col + name.len()) as u32,
                                    };
                                    
                                    let range = Range { start, end };
                                    let file_uri = path_to_uri(&resolved_path);
                                    info!("Creating location with URI: {}", file_uri);
                                    
                                    match Url::parse(&file_uri) {
                                        Ok(uri) => {
                                            info!("Successfully parsed URI");
                                            return Some(Location { uri, range });
                                        }
                                        Err(e) => {
                                            info!("Failed to parse URI: {:?}", e);
                                            return None;
                                        }
                                    }
                                }
                            }
                        } else {
                            info!("Failed to load macros from {:?}", resolved_path);
                        }
                    } else {
                        info!("Failed to resolve path for {}", import_path);
                    }
                    break;
                }
            }
            
            return None;
        }
        
        // Find the definition in current file
        for (name, span, _body) in &doc.macro_definitions {
            if name == macro_ref {
                // Convert span to LSP Location
                let start = Position {
                    line: (span.line - 1) as u32,
                    character: (span.col - 1) as u32,
                };
                // For now, use start position for end too (we could improve this later)
                let end = Position {
                    line: (span.line - 1) as u32,
                    character: (span.col + name.len()) as u32,
                };
                
                let range = Range { start, end };
                
                // We need the document URI - for now assume same document
                // In a real implementation, we'd need to track which document we're in
                return Some(Location {
                    uri: Url::parse("file:///dummy").unwrap(), // This will be set by the caller
                    range,
                });
            }
        }
    }
    
    None
}
