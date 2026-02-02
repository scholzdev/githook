use crate::document::DocumentState;
use crate::import_resolver::{load_imported_macros, path_to_uri, resolve_import_path};
use tower_lsp::lsp_types::*;
use tracing::info;

pub fn get_definition(
    doc: &DocumentState,
    position: Position,
    current_uri: &str,
) -> Option<Location> {
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

    let word_start = before_cursor
        .rfind(|c: char| c.is_whitespace() || c == '{' || c == '}')
        .map(|pos| pos + 1)
        .unwrap_or(0);

    let after_cursor = &line[char_idx..];
    let word_end = char_idx
        + after_cursor
            .find(|c: char| c.is_whitespace() || c == '{' || c == '}')
            .unwrap_or(after_cursor.len());

    let word = &line[word_start..word_end];

    if let Some(macro_ref) = word.strip_prefix('@') {
        if let Some(dot_pos) = macro_ref.find('.') {
            let namespace = &macro_ref[..dot_pos];
            let macro_name = &macro_ref[dot_pos + 1..];

            info!("Looking for namespaced macro: {}.{}", namespace, macro_name);

            let imports = doc
                .ast
                .as_ref()
                .map(|ast| crate::ast_utils::extract_imports(ast))
                .unwrap_or_default();

            for import_info in &imports {
                if import_info.alias.as_deref() == Some(namespace) {
                    info!("Found import: {} -> {}", namespace, import_info.path);

                    if let Some(resolved_path) = resolve_import_path(current_uri, &import_info.path)
                    {
                        info!("Resolved path: {:?}", resolved_path);

                        if let Ok(imported_macros) = load_imported_macros(&resolved_path) {
                            info!("Loaded {} macros", imported_macros.len());

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
                        info!("Failed to resolve path for {}", import_info.path);
                    }
                    break;
                }
            }

            return None;
        }

        let macros = doc
            .ast
            .as_ref()
            .map(|ast| crate::ast_utils::extract_macros(ast))
            .unwrap_or_default();

        for macro_info in &macros {
            if macro_info.name == macro_ref {
                let start = Position {
                    line: 0,
                    character: 0,
                };
                let end = Position {
                    line: 0,
                    character: macro_info.name.len() as u32,
                };

                let range = Range { start, end };

                return Some(Location {
                    uri: Url::parse("file:///dummy").unwrap(),
                    range,
                });
            }
        }
    }

    None
}
