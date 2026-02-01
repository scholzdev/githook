use tower_lsp::lsp_types::*;
use crate::document::DocumentState;

pub fn find_references(doc: &DocumentState, position: Position, _include_declaration: bool) -> Vec<Location> {
    let mut locations = Vec::new();
    
    let line_idx = position.line as usize;
    let lines: Vec<&str> = doc.text.lines().collect();
    
    if line_idx >= lines.len() {
        return locations;
    }
    
    let line = lines[line_idx];
    let char_idx = position.character as usize;
    
    if char_idx > line.len() {
        return locations;
    }
    
    let word_start = line[..char_idx].rfind(|c: char| !c.is_alphanumeric() && c != '_' && c != '@' && c != ':')
        .map(|p| p + 1)
        .unwrap_or(0);
    
    let word_end = char_idx + line[char_idx..].find(|c: char| !c.is_alphanumeric() && c != '_' && c != ':')
        .unwrap_or(line[char_idx..].len());
    
    let word = &line[word_start..word_end];
    
    if let Some(macro_ref) = word.strip_prefix('@') {
        let macro_name = if let Some(colon_pos) = macro_ref.find(':') {
            &macro_ref[colon_pos + 1..]
        } else {
            macro_ref
        };
        
        for (line_num, line_text) in doc.text.lines().enumerate() {
            let mut start_pos = 0;
            while let Some(pos) = line_text[start_pos..].find(&format!("@{}", macro_name)) {
                let actual_pos = start_pos + pos;
                
                let after_pos = actual_pos + 1 + macro_name.len();
                let is_complete = after_pos >= line_text.len() 
                    || !line_text.chars().nth(after_pos).unwrap().is_alphanumeric();
                
                if is_complete {
                    locations.push(Location {
                        uri: Url::parse("file:///dummy").unwrap(),
                        range: Range {
                            start: Position {
                                line: line_num as u32,
                                character: actual_pos as u32,
                            },
                            end: Position {
                                line: line_num as u32,
                                character: (actual_pos + 1 + macro_name.len()) as u32,
                            },
                        },
                    });
                }
                
                start_pos = actual_pos + 1;
            }
        }
    }
    
    locations
}
