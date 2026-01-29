use tower_lsp::lsp_types::*;
use crate::document::DocumentState;

pub fn get_completions(doc: &DocumentState, position: Position) -> Vec<CompletionItem> {
    let mut completions = Vec::new();
    
    if let Some(context) = get_context(doc, position) {
        match context {
            GithookCompletionContext::MacroCall => {
                // TODO: Extract macros from AST
                return completions;
            }
            GithookCompletionContext::PropertyAccess(prefix) => {
                return get_property_completions(&prefix);
            }
            GithookCompletionContext::Normal => {}
        }
    }

    for (label, detail) in &[
        ("run", "Execute shell command"),
        ("block", "Block commit"),
        ("warn", "Show warning"),
        ("if", "Conditional"),
        ("else", "Alternative"),
        ("foreach", "Loop"),
        ("break", "Exit loop"),
        ("continue", "Next iteration"),
        ("let", "Variable"),
        ("match", "Pattern match"),
    ] {
        completions.push(CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some(detail.to_string()),
            ..Default::default()
        });
    }

    for label in &["git", "true", "false", "null"] {
        completions.push(CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::CONSTANT),
            ..Default::default()
        });
    }

    completions
}

#[derive(Debug, PartialEq)]
enum GithookCompletionContext {
    Normal,
    MacroCall,
    PropertyAccess(String),
}

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
    
    if let Some(dot_pos) = before_cursor.rfind('.') {
        let before_dot = &before_cursor[..dot_pos];
        let ident_start = before_dot.rfind(|c: char| {
            c.is_whitespace() || "({[,=!<>".contains(c)
        }).map(|pos| pos + 1).unwrap_or(0);
        
        let prefix = before_dot[ident_start..].trim();
        
        if !prefix.is_empty() {
            return Some(GithookCompletionContext::PropertyAccess(prefix.to_string()));
        }
    }
    
    if before_cursor.trim_end().ends_with('@') {
        return Some(GithookCompletionContext::MacroCall);
    }
    
    Some(GithookCompletionContext::Normal)
}

fn get_property_completions(prefix: &str) -> Vec<CompletionItem> {
    let mut completions = Vec::new();
    
    let properties: &[(&str, &str)] = match prefix {
        "git" => &[
            ("staged_files", "Staged files array"),
            ("all_files", "All files array"),
            ("branch", "Branch object"),
            ("commit", "Commit object"),
            ("author", "Author object"),
            ("remote", "Remote object"),
            ("stats", "Stats object"),
            ("is_merge_commit", "Boolean"),
            ("has_conflicts", "Boolean"),
        ],
        "git.branch" | "branch" => &[("name", "Branch name")],
        "git.commit" | "commit" => &[("message", "Commit message"), ("hash", "Commit hash")],
        "git.author" | "author" => &[("name", "Author name"), ("email", "Author email")],
        "git.remote" | "remote" => &[("name", "Remote name"), ("url", "Remote URL")],
        "git.stats" | "stats" => &[
            ("files_changed", "Files changed"),
            ("additions", "Lines added"),
            ("deletions", "Lines deleted"),
            ("modified_lines", "Total modified"),
        ],
        p if p.starts_with("f") || p.starts_with("file") => &[
            ("name", "Filename"),
            ("path", "Path object"),
            ("basename", "Base name"),
            ("extension", "Extension"),
            ("dirname", "Directory"),
            ("size", "File size"),
            ("exists()", "Check exists"),
            ("contains(pattern)", "Contains text"),
            ("starts_with(prefix)", "Starts with"),
            ("ends_with(suffix)", "Ends with"),
        ],
        p if p.ends_with(".path") || p == "path" => &[
            ("string", "Path as string"),
            ("basename", "Base name"),
            ("extension", "Extension"),
            ("parent", "Parent directory"),
            ("filename", "Filename"),
            ("join(suffix)", "Join path"),
        ],
        _ => &[],
    };
    
    for (name, detail) in properties {
        completions.push(CompletionItem {
            label: name.to_string(),
            kind: Some(if name.contains('(') {
                CompletionItemKind::METHOD
            } else {
                CompletionItemKind::PROPERTY
            }),
            detail: Some(detail.to_string()),
            ..Default::default()
        });
    }
    
    completions
}
