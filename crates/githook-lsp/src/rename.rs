use tower_lsp::lsp_types::*;
use crate::document::DocumentState;
use crate::references::find_references;

/// Prepare rename operation (validate if renaming is allowed)
pub fn prepare_rename(doc: &DocumentState, position: Position) -> Option<Range> {
    let line_idx = position.line as usize;
    let lines: Vec<&str> = doc.text.lines().collect();
    
    if line_idx >= lines.len() {
        return None;
    }
    
    let line = lines[line_idx];
    let char_idx = position.character as usize;
    
    if char_idx > line.len() {
        return None;
    }
    
    // Find word boundaries
    let word_start = line[..char_idx].rfind(|c: char| !c.is_alphanumeric() && c != '_' && c != '@' && c != ':')
        .map(|p| p + 1)
        .unwrap_or(0);
    
    let word_end = char_idx + line[char_idx..].find(|c: char| !c.is_alphanumeric() && c != '_' && c != ':')
        .unwrap_or(line[char_idx..].len());
    
    let word = &line[word_start..word_end];
    
    // Only allow rename on macros
    if let Some(macro_ref) = word.strip_prefix('@') {
        // Don't allow rename on namespaced macros (would need cross-file edit)
        if macro_ref.contains(':') {
            return None;
        }
        
        Some(Range {
            start: Position {
                line: line_idx as u32,
                character: word_start as u32,
            },
            end: Position {
                line: line_idx as u32,
                character: word_end as u32,
            },
        })
    } else {
        None
    }
}

/// Execute rename operation
pub fn execute_rename(doc: &DocumentState, position: Position, new_name: String) -> Option<WorkspaceEdit> {
    // Find all references
    let locations = find_references(doc, position, true);
    
    if locations.is_empty() {
        return None;
    }
    
    // Create text edits for all references
    let mut changes = std::collections::HashMap::new();
    let uri = locations[0].uri.clone();
    
    let edits: Vec<TextEdit> = locations.iter().map(|loc| {
        TextEdit {
            range: loc.range,
            new_text: format!("@{}", new_name),
        }
    }).collect();
    
    changes.insert(uri, edits);
    
    Some(WorkspaceEdit {
        changes: Some(changes),
        document_changes: None,
        change_annotations: None,
    })
}
