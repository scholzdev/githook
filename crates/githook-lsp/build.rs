use std::collections::HashMap;
use std::fs;

fn main() {
    println!("cargo:rerun-if-changed=../githook-eval/src/contexts.rs");
    
    let content = fs::read_to_string("../githook-eval/src/contexts.rs")
        .expect("Failed to read contexts.rs");
    
    let (properties, methods) = extract_docs(&content);
    
    let db = serde_json::json!({
        "properties": properties,
        "methods": methods,
    });
    
    let src_path = "src/generated_docs.json";
    fs::write(src_path, serde_json::to_string_pretty(&db).unwrap())
        .expect("Failed to write docs.json");
}

fn extract_docs(content: &str) -> (HashMap<String, DocEntry>, HashMap<String, DocEntry>) {
    let mut properties = HashMap::new();
    let mut methods = HashMap::new();
    
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;
    
    while i < lines.len() {
        let line = lines[i].trim();
        
        if line.starts_with("#[docs(")
            && let Some(doc_entry) = parse_docs_attr(line) {
                let mut j = i + 1;
                while j < lines.len() {
                    let next_line = lines[j].trim();
                    
                    if next_line.starts_with("#[property]") {
                        properties.insert(doc_entry.name.clone(), doc_entry.clone());
                        break;
                    } else if next_line.starts_with("#[method]") {
                        methods.insert(doc_entry.name.clone(), doc_entry.clone());
                        break;
                    } else if next_line.starts_with("pub fn") || next_line.starts_with("fn ") {
                        break;
                    }
                    
                    j += 1;
                }
            }
        
        i += 1;
    }
    
    (properties, methods)
}

#[derive(Debug, Clone, serde::Serialize)]
struct DocEntry {
    name: String,
    description: String,
    example: String,
}

fn parse_docs_attr(line: &str) -> Option<DocEntry> {
    let mut name = None;
    let mut description = None;
    let mut example = None;
    
    let content = line.strip_prefix("#[docs(")?
        .strip_suffix(")]")?;
    
    let mut current_key = String::new();
    let mut current_value = String::new();
    let mut in_quotes = false;
    let mut after_equals = false;
    let mut escape_next = false;
    
    for ch in content.chars() {
        if escape_next {
            current_value.push(ch);
            escape_next = false;
            continue;
        }
        
        if ch == '\\' && in_quotes {
            escape_next = true;
            continue;
        } else if ch == '"' {
            in_quotes = !in_quotes;
        } else if ch == '=' && !in_quotes {
            after_equals = true;
        } else if ch == ',' && !in_quotes {
            let key = current_key.trim();
            let value = current_value.trim();
            match key {
                "name" => name = Some(value.to_string()),
                "description" => description = Some(value.to_string()),
                "example" => example = Some(value.to_string()),
                _ => {}
            }
            current_key.clear();
            current_value.clear();
            after_equals = false;
        } else if after_equals {
            current_value.push(ch);
        } else {
            current_key.push(ch);
        }
    }
    
    if !current_key.is_empty() {
        let key = current_key.trim();
        let value = current_value.trim();
        match key {
            "name" => name = Some(value.to_string()),
            "description" => description = Some(value.to_string()),
            "example" => example = Some(value.to_string()),
            _ => {}
        }
    }
    
    Some(DocEntry {
        name: name?,
        description: description?,
        example: example?,
    })
}
