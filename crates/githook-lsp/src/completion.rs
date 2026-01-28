use tower_lsp::lsp_types::*;
use crate::document::DocumentState;

/// Get completion items at the given position
pub fn get_completions(doc: &DocumentState, position: Position) -> Vec<CompletionItem> {
    let mut completions = Vec::new();
    
    // Check context
    if let Some(context) = get_context(doc, position) {
        match context {
            GithookCompletionContext::MacroCall => {
                // Only suggest macros
                for macro_name in &doc.macros {
                    completions.push(CompletionItem {
                        label: macro_name.clone(),
                        kind: Some(CompletionItemKind::FUNCTION),
                        detail: Some("Macro call".to_string()),
                        insert_text: Some(macro_name.clone()),
                        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                        ..Default::default()
                    });
                }
                return completions;
            }
            GithookCompletionContext::Placeholder(prefix) => {
                return get_placeholder_completions(&prefix);
            }
            GithookCompletionContext::Normal => {
                // Fall through to normal completions
            }
        }
    }

    // Keywords
    let keywords = vec![
        ("run", "Execute a shell command", CompletionItemKind::KEYWORD),
        ("block", "Block the commit with a message", CompletionItemKind::KEYWORD),
        ("allow", "Allow a specific command", CompletionItemKind::KEYWORD),
        ("when", "Conditional execution", CompletionItemKind::KEYWORD),
        ("match", "Pattern matching", CompletionItemKind::KEYWORD),
        ("foreach", "Iterate over files", CompletionItemKind::KEYWORD),
        ("parallel", "Run commands in parallel", CompletionItemKind::KEYWORD),
        ("group", "Group rules together", CompletionItemKind::KEYWORD),
        ("macro", "Define a reusable macro", CompletionItemKind::KEYWORD),
        ("use", "Import from stdlib", CompletionItemKind::KEYWORD),
        ("import", "Import from file", CompletionItemKind::KEYWORD),
        ("let", "Define a variable", CompletionItemKind::KEYWORD),
        ("warn_if", "Warn if condition is true", CompletionItemKind::KEYWORD),
        ("block_if", "Block if condition is true", CompletionItemKind::KEYWORD),
    ];

    for (label, detail, kind) in keywords {
        completions.push(CompletionItem {
            label: label.to_string(),
            kind: Some(kind),
            detail: Some(detail.to_string()),
            insert_text: Some(format!("{} ", label)),
            insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
            ..Default::default()
        });
    }

    // File Collections (for foreach loops)
    let file_collections = vec![
        ("staged_files", "All staged files", "foreach file in staged_files matching \"*.rs\""),
        ("all_files", "All files in repo", "foreach file in all_files matching \"*.md\""),
    ];

    for (label, detail, example) in file_collections {
        completions.push(CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::CONSTANT),
            detail: Some(detail.to_string()),
            documentation: Some(Documentation::String(format!("Example: {}", example))),
            ..Default::default()
        });
    }

    // String Properties
    let string_properties = vec![
        ("content", "File content", "content matches \"pattern\""),
        ("staged_content", "Staged file content", "staged_content contains \"TODO\""),
        ("diff", "Staged changes diff", "diff matches \"^-.*old\""),
        ("branch_name", "Git branch name", "branch_name matches \"^main$\""),
        ("commit_message", "Commit message text", "commit_message contains \"fix\""),
        ("author_email", "Git author email", "author_email matches \"@company.com\""),
        ("extension", "File extension", "extension == \".rs\""),
        ("filename", "File name with extension", "filename == \"main.rs\""),
        ("basename", "File name without extension", "basename == \"main\""),
        ("dirname", "Directory path", "dirname matches \"^src/\""),
    ];

    for (label, detail, example) in string_properties {
        completions.push(CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::PROPERTY),
            detail: Some(detail.to_string()),
            documentation: Some(Documentation::String(format!("Example: {}", example))),
            ..Default::default()
        });
    }

    // Numeric Properties
    let numeric_properties = vec![
        ("file_size", "File size in bytes", "file_size > 1048576"),
        ("modified_lines", "Changed lines in diff", "modified_lines > 500"),
        ("files_changed", "Number of changed files", "files_changed == 5"),
        ("additions", "Added lines", "additions >= 10"),
        ("deletions", "Deleted lines", "deletions < 100"),
        ("commits_ahead", "Commits ahead of remote", "commits_ahead > 0"),
    ];

    for (label, detail, example) in numeric_properties {
        completions.push(CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::PROPERTY),
            detail: Some(detail.to_string()),
            documentation: Some(Documentation::String(format!("Example: {}", example))),
            ..Default::default()
        });
    }

    // Boolean Properties
    let boolean_properties = vec![
        ("author_set", "Git user.name is set", "block_if not author_set"),
        ("author_email_set", "Git user.email is set", "block_if not author_email_set"),
        ("author_missing", "Git author not configured", "block_if author_missing"),
        ("contains_secrets", "Secrets/credentials detected", "block_if contains_secrets"),
        ("file_exists", "File exists check", "block_if not file_exists"),
    ];

    for (label, detail, example) in boolean_properties {
        completions.push(CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::PROPERTY),
            detail: Some(detail.to_string()),
            documentation: Some(Documentation::String(format!("Example: {}", example))),
            ..Default::default()
        });
    }

    // Operators
    let operators = vec![
        ("matches", "Regex match operator", "content matches \"^TODO\""),
        ("contains", "String contains operator", "content contains \"panic!\""),
        ("==", "Equality operator", "file_size == 0"),
        ("!=", "Not equals operator", "branch_name != \"main\""),
        (">", "Greater than operator", "file_size > 1000000"),
        (">=", "Greater or equal operator", "modified_lines >= 500"),
        ("<", "Less than operator", "additions < 10"),
        ("<=", "Less or equal operator", "deletions <= 100"),
        ("and", "Logical AND", "file_size > 0 and content matches \"test\""),
        ("or", "Logical OR", "extension == \".rs\" or extension == \".toml\""),
        ("not", "Logical NOT / negation", "not author_set"),
        ("in", "List membership", "extension in {forbidden}"),
        ("matching", "Pattern for foreach", "foreach file in staged_files matching \"*.rs\""),
    ];

    for (label, detail, example) in operators {
        completions.push(CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::OPERATOR),
            detail: Some(detail.to_string()),
            documentation: Some(Documentation::String(format!("Example: {}", example))),
            ..Default::default()
        });
    }

    completions
}

#[derive(Debug, PartialEq)]
enum GithookCompletionContext {
    Normal,
    MacroCall,
    Placeholder(String), // Prefix like "file", "git", "env", etc.
}

/// Get the current completion context
fn get_context(doc: &DocumentState, position: Position) -> Option<GithookCompletionContext> {
    let line_idx = position.line as usize;
    let lines: Vec<&str> = doc.text.lines().collect();
    
    if line_idx >= lines.len() {
        return Some(GithookCompletionContext::Normal);
    }
    
    let line = lines[line_idx];
    let char_idx = position.character as usize;
    
    if char_idx == 0 || char_idx > line.len() {
        return Some(GithookCompletionContext::Normal);
    }
    
    let before_cursor = &line[..char_idx];
    
    // Check for placeholder: {prefix: or {prefix
    if let Some(brace_pos) = before_cursor.rfind('{') {
        let after_brace = &before_cursor[brace_pos + 1..];
        
        // No closing brace yet?
        if !after_brace.contains('}') {
            // Check if we have a colon
            if let Some(colon_pos) = after_brace.find(':') {
                let prefix = after_brace[..colon_pos].trim();
                return Some(GithookCompletionContext::Placeholder(prefix.to_string()));
            } else {
                // Just "{prefix" - suggest namespace
                return Some(GithookCompletionContext::Placeholder(after_brace.trim().to_string()));
            }
        }
    }
    
    // Check for macro call: @prefix
    let word_start = before_cursor.rfind(|c: char| c.is_whitespace() || c == '{' || c == '}')
        .map(|pos| pos + 1)
        .unwrap_or(0);
    
    let current_word = &before_cursor[word_start..];
    
    if current_word.starts_with('@') {
        return Some(GithookCompletionContext::MacroCall);
    }
    
    Some(GithookCompletionContext::Normal)
}

/// Get placeholder completions based on prefix
fn get_placeholder_completions(prefix: &str) -> Vec<CompletionItem> {
    let mut completions = Vec::new();
    
    match prefix {
        "" => {
            // No prefix yet - suggest all namespaces
            let namespaces = vec![
                ("file", "File context placeholders"),
                ("git", "Git context placeholders"),
                ("commit", "Commit context placeholders"),
                ("repo", "Repository context placeholders"),
                ("system", "System context placeholders"),
                ("diff", "Diff context placeholders"),
                ("time", "Time context placeholders"),
                ("env", "Environment variables"),
            ];
            
            for (ns, detail) in namespaces {
                completions.push(CompletionItem {
                    label: format!("{}:", ns),
                    kind: Some(CompletionItemKind::MODULE),
                    detail: Some(detail.to_string()),
                    insert_text: Some(format!("{}:", ns)),
                    ..Default::default()
                });
            }
        }
        "file" => {
            let placeholders = vec![
                ("path", "Complete file path"),
                ("name", "Filename with extension"),
                ("basename", "Filename without extension"),
                ("extension", "File extension"),
                ("dirname", "Directory path"),
                ("size", "File size in bytes"),
            ];
            
            for (name, detail) in placeholders {
                completions.push(CompletionItem {
                    label: name.to_string(),
                    kind: Some(CompletionItemKind::FIELD),
                    detail: Some(detail.to_string()),
                    insert_text: Some(format!("{}}}",name)),
                    ..Default::default()
                });
            }
        }
        "git" => {
            let placeholders = vec![
                ("branch", "Current branch name"),
                ("commit_message", "Commit message"),
                ("author_name", "Author name"),
                ("author_email", "Author email"),
                ("repo_root", "Repository root path"),
                ("remote_url", "Remote URL"),
            ];
            
            for (name, detail) in placeholders {
                completions.push(CompletionItem {
                    label: name.to_string(),
                    kind: Some(CompletionItemKind::FIELD),
                    detail: Some(detail.to_string()),
                    insert_text: Some(format!("{}}}",name)),
                    ..Default::default()
                });
            }
        }
        "commit" => {
            let placeholders = vec![
                ("message", "Commit message"),
                ("files", "Number of changed files"),
                ("additions", "Added lines"),
                ("deletions", "Deleted lines"),
            ];
            
            for (name, detail) in placeholders {
                completions.push(CompletionItem {
                    label: name.to_string(),
                    kind: Some(CompletionItemKind::FIELD),
                    detail: Some(detail.to_string()),
                    insert_text: Some(format!("{}}}",name)),
                    ..Default::default()
                });
            }
        }
        "diff" => {
            let placeholders = vec![
                ("added", "Added lines (only + lines)"),
                ("stats", "Diff statistics"),
            ];
            
            for (name, detail) in placeholders {
                completions.push(CompletionItem {
                    label: name.to_string(),
                    kind: Some(CompletionItemKind::FIELD),
                    detail: Some(detail.to_string()),
                    insert_text: Some(format!("{}}}",name)),
                    ..Default::default()
                });
            }
        }
        "repo" => {
            let placeholders = vec![
                ("root", "Repository root path"),
                ("name", "Repository name"),
                ("has_remote", "Has remote (true/false)"),
            ];
            
            for (name, detail) in placeholders {
                completions.push(CompletionItem {
                    label: name.to_string(),
                    kind: Some(CompletionItemKind::FIELD),
                    detail: Some(detail.to_string()),
                    insert_text: Some(format!("{}}}",name)),
                    ..Default::default()
                });
            }
        }
        "system" => {
            let placeholders = vec![
                ("os", "Operating system"),
                ("arch", "Architecture"),
                ("user", "Username"),
                ("hostname", "Machine hostname"),
            ];
            
            for (name, detail) in placeholders {
                completions.push(CompletionItem {
                    label: name.to_string(),
                    kind: Some(CompletionItemKind::FIELD),
                    detail: Some(detail.to_string()),
                    insert_text: Some(format!("{}}}",name)),
                    ..Default::default()
                });
            }
        }
        "time" => {
            let placeholders = vec![
                ("hour", "Current hour (0-23)"),
                ("day", "Day of week"),
                ("is_weekend", "Is weekend (true/false)"),
                ("is_night", "Is night time (true/false)"),
            ];
            
            for (name, detail) in placeholders {
                completions.push(CompletionItem {
                    label: name.to_string(),
                    kind: Some(CompletionItemKind::FIELD),
                    detail: Some(detail.to_string()),
                    insert_text: Some(format!("{}}}",name)),
                    ..Default::default()
                });
            }
        }
        "env" => {
            // For env, suggest common env vars
            let common_vars = vec![
                ("CI", "CI environment"),
                ("GITHUB_ACTIONS", "GitHub Actions"),
                ("USER", "Current user"),
                ("HOME", "Home directory"),
                ("PATH", "PATH variable"),
            ];
            
            for (name, detail) in common_vars {
                completions.push(CompletionItem {
                    label: name.to_string(),
                    kind: Some(CompletionItemKind::VARIABLE),
                    detail: Some(detail.to_string()),
                    insert_text: Some(format!("{}}}",name)),
                    ..Default::default()
                });
            }
        }
        _ => {
            // Unknown prefix - no suggestions
        }
    }
    
    completions
}
