use tower_lsp::lsp_types::*;
use crate::document::DocumentState;
use crate::import_resolver::{resolve_import_path, load_imported_macros};
use crate::docs::{get_property_doc, get_method_doc};
use tracing::info;

pub fn get_hover(doc: &DocumentState, position: Position, current_uri: &str) -> Option<Hover> {
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
    
    let word_start = line[..char_idx].rfind(|c: char| {
        !c.is_alphanumeric() && c != '_' && c != '@' && c != '.' && c != '(' && c != ')'
    }).map(|p| p + 1).unwrap_or(0);
    
    let word_end = char_idx + line[char_idx..].find(|c: char| {
        !c.is_alphanumeric() && c != '_' && c != '.' && c != '(' && c != ')'
    }).unwrap_or(line[char_idx..].len());
    
    let word = &line[word_start..word_end].trim_start_matches(['"', '\'', '.']);
    
    info!("Hover word: '{}' at position {}:{}", word, position.line, position.character);
    
    if word.contains('.') {
        if let Some(last_dot) = word.rfind('.') {
            let method_part = &word[last_dot + 1..];
            let method_name = method_part.trim_end_matches("()");
            
            if let Some(content) = get_method_hover(method_name) {
                return Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: content,
                    }),
                    range: None,
                });
            }
            
            if let Some(content) = get_property_hover(method_name) {
                return Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: content,
                    }),
                    range: None,
                });
            }
        }
        
        if let Some(content) = get_property_chain_hover(word) {
            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: content,
                }),
                range: None,
            });
        }
    }
    
    if !word.contains('.') && !word.starts_with('@') {
        let method_name = word.trim_end_matches("()");
        if let Some(content) = get_method_hover(method_name) {
            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: content,
                }),
                range: None,
            });
        }
    }
    
    if let Some(macro_ref) = word.strip_prefix('@') {
        info!("Hover on macro: {}", macro_ref);
        
        if let Some(dot_pos) = macro_ref.find('.') {
            let namespace = &macro_ref[..dot_pos];
            let macro_name = &macro_ref[dot_pos + 1..];
            
            info!("Namespaced macro: {}.{}", namespace, macro_name);
            
            let imports = doc.ast.as_ref()
                .map(|ast| crate::ast_utils::extract_imports(ast))
                .unwrap_or_default();
            
            for import_info in &imports {
                if import_info.alias.as_deref() == Some(namespace) {
                    info!("Found import: {} -> {}", namespace, import_info.path);
                    
                    if let Some(resolved_path) = resolve_import_path(current_uri, &import_info.path) {
                        info!("Resolved path: {:?}", resolved_path);
                        
                        if let Ok(imported_macros) = load_imported_macros(&resolved_path) {
                            info!("Loaded {} macros from import", imported_macros.len());
                            
                            for (name, _span, body) in imported_macros {
                                if name == macro_name {
                                    info!("Found macro {} in imports", name);
                                    let body_str = format_macro_body(&body);
                                    let alias_display = import_info.alias.as_ref().unwrap_or(&import_info.path);
                                    return Some(create_hover(&format!(
                                        "**Macro:** `{}` (from `{}`)\n\n```githook\nmacro {} {{\n{}}}\n```",
                                        name, alias_display, name, body_str
                                    )));
                                }
                            }
                        } else {
                            info!("Failed to load macros from {:?}", resolved_path);
                        }
                    } else {
                        info!("Failed to resolve import path: {}", import_info.path);
                    }
                    break;
                }
            }
            
            info!("Macro not found in imports");
            return Some(create_hover(&format!("**Macro:** `@{}:{}`\n\nNamespaced macro (not resolved)", namespace, macro_name)));
        }
        
        let macro_name = macro_ref;
        
        let macros = doc.ast.as_ref()
            .map(|ast| crate::ast_utils::extract_macros(ast))
            .unwrap_or_default();
        
        for macro_info in &macros {
            if macro_info.name == macro_name {
                let params = macro_info.params.join(", ");
                return Some(create_hover(&format!(
                    "**Macro:** `{}`(`{}`)\n\nUser-defined macro",
                    macro_info.name, params
                )));
            }
        }
        
        return Some(create_hover(&format!("**Macro:** `@{}`\n\nUser-defined macro (not found)", macro_name)));
    }
    
    let keyword_docs = get_keyword_documentation(word);
    if let Some(docs) = keyword_docs {
        return Some(create_hover(docs));
    }
    
    let property_docs = get_property_documentation(word);
    if let Some(docs) = property_docs {
        return Some(create_hover(docs));
    }
    
    None
}

fn create_hover(markdown: &str) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: markdown.to_string(),
        }),
        range: None,
    }
}

fn get_keyword_documentation(keyword: &str) -> Option<&'static str> {
    match keyword {
        "run" => Some("**run** `\"command\"`\n\nExecute a shell command.\n\n**Example:**\n```githook\nrun \"cargo test\"\nrun \"npm run lint\"\n```"),
        "block" => Some("**block** `\"message\"`\n\nBlock the commit with a message.\n\n**Example:**\n```githook\nblock \"Direct commits not allowed\"\n```"),
        "block_if" => Some("**block_if** `<condition>` **message** `\"text\"`\n\nBlock commit if condition is true.\n\n**Example:**\n```githook\nblock_if branch_name == \"main\" message \"No commits to main\"\nblock_if file_size > 1000000 message \"File too large\"\n```"),
        "warn_if" => Some("**warn_if** `<condition>` **message** `\"text\"`\n\nWarn if condition is true (non-blocking).\n\n**Example:**\n```githook\nwarn_if modified_lines > 500 message \"Large changeset\"\n```"),
        "when" => Some("**when** `<condition>` **{** ... **}**\n\nConditional execution block.\n\n**Example:**\n```githook\nwhen branch_name == \"main\" {\n    run \"npm test\"\n    block_if content matches \"TODO\"\n}\n```"),
        "foreach" => Some("**foreach** `file` **in** `<collection>` **matching** `\"pattern\"` **{** ... **}**\n\nIterate over files.\n\n**Example:**\n```githook\nforeach file in git.files.staged matching \"*.rs\" {\n    block_if content matches \"panic!\"\n}\n```"),
        "match" => Some("**match** `<value>` **{** ... **}**\n\nPattern matching.\n\n**Example:**\n```githook\nmatch file {\n    \"*.rs\" -> run \"cargo clippy\"\n    \"*.js\" -> run \"npm run lint\"\n    _ -> run \"echo 'unknown'\"\n}\n```"),
        "macro" => Some("**macro** `name` **{** ... **}**\n\nDefine a reusable macro.\n\n**Example:**\n```githook\nmacro check_main {\n    block_if branch_name == \"main\"\n}\n\n@check_main  # Call the macro\n```"),
        "let" => Some("**let** `name` **=** `[...]`\n\nDefine a variable (string list).\n\n**Example:**\n```githook\nlet forbidden = [\".txt\", \".zip\"]\n\nforeach file in git.files.staged {\n    block_if {file:extension} in {forbidden}\n}\n```"),
        "use" => Some("**use** `@namespace/package`\n\nImport from remote package (GitHub).\n\n**Example:**\n```githook\nuse @preview/security\n\n@no_secrets\n```"),
        "import" => Some("**import** `\"path/to/file.ghook\"`\n\nImport from local file.\n\n**Example:**\n```githook\nimport \"./common.ghook\"\n```"),
        _ => None,
    }
}

fn get_property_documentation(property: &str) -> Option<&'static str> {
    match property {
        "branch_name" => Some("**branch_name**: String\n\nCurrent Git branch name.\n\n**Example:**\n```githook\nblock_if branch_name == \"main\"\nblock_if branch_name matches \"^feature/\"\n```"),
        "content" => Some("**content**: String\n\nStaged file content.\n\n**Example:**\n```githook\nblock_if content matches \"TODO\"\nblock_if content contains \"panic!\"\n```"),
        "staged_content" => Some("**staged_content**: String\n\nStaged file content (alias for content).\n\n**Example:**\n```githook\nblock_if staged_content matches \"console.log\"\n```"),
        "diff" => Some("**diff**: String\n\nStaged changes diff.\n\n**Example:**\n```githook\nblock_if diff matches \"^-.*password\"\n```"),
        "commit_message" => Some("**commit_message**: String\n\nCommit message text.\n\n**Example:**\n```githook\nblock_if commit_message contains \"WIP\"\n```"),
        "file_size" => Some("**file_size**: Number\n\nFile size in bytes.\n\n**Example:**\n```githook\nblock_if file_size > 1048576 message \"File > 1MB\"\n```"),
        "modified_lines" => Some("**modified_lines**: Number\n\nChanged lines in diff.\n\n**Example:**\n```githook\nwarn_if modified_lines > 500\n```"),
        "files_changed" => Some("**files_changed**: Number\n\nNumber of changed files.\n\n**Example:**\n```githook\nwarn_if files_changed > 20\n```"),
        "additions" => Some("**additions**: Number\n\nAdded lines.\n\n**Example:**\n```githook\nblock_if additions > 1000\n```"),
        "deletions" => Some("**deletions**: Number\n\nDeleted lines.\n\n**Example:**\n```githook\nblock_if deletions > 500\n```"),
        "commits_ahead" => Some("**commits_ahead**: Number\n\nCommits ahead of remote.\n\n**Example:**\n```githook\nblock_if commits_ahead > 5\n```"),
        "author_set" => Some("**author_set**: Boolean\n\nGit user.name is configured.\n\n**Example:**\n```githook\nblock_if not author_set message \"Configure git user\"\n```"),
        "author_email_set" => Some("**author_email_set**: Boolean\n\nGit user.email is configured.\n\n**Example:**\n```githook\nblock_if not author_email_set\n```"),
        "contains_secrets" => Some("**contains_secrets**: Boolean\n\nSecrets/credentials detected.\n\n**Example:**\n```githook\nblock_if contains_secrets message \"Secrets found!\"\n```"),
        "git.files" => Some("**git.files**: FilesCollection\n\nFile collections.\n\n**Example:**\n```githook\nforeach file in git.files.staged {\n    print file.name\n}\n```"),
        "git.diff" => Some("**git.diff**: DiffCollection\n\nDiff lines for staged changes.\n\n**Example:**\n```githook\nif git.diff.added_lines.any(|line| line.contains(\"console.log\")) {\n    warn \"Debug statement found\"\n}\n```"),
        "git.diff.added_lines" => Some("**git.diff.added_lines**: Array<String>\n\nAll added lines in staged changes.\n\n**Example:**\n```githook\nlet debug_count = git.diff.added_lines.filter(|line| line.contains(\"print\")).length\n```"),
        "git.diff.removed_lines" => Some("**git.diff.removed_lines**: Array<String>\n\nAll removed lines in staged changes.\n\n**Example:**\n```githook\nif git.diff.removed_lines.any(|line| line.contains(\"TODO\")) {\n    print \"TODO removed\"\n}\n```"),
        "git.merge" => Some("**git.merge**: MergeContext\n\nMerge information (post-merge hook).\n\n**Example:**\n```githook\nprint \"Merged \" + git.merge.source + \" into \" + git.merge.target\n```"),
        "git.merge.source" => Some("**git.merge.source**: String\n\nSource branch/commit that was merged in.\n\n**Example:**\n```githook\nif git.merge.source.contains(\"feature\") {\n    print \"Feature branch merged\"\n}\n```"),
        "git.merge.target" => Some("**git.merge.target**: String\n\nTarget branch that was merged into.\n\n**Example:**\n```githook\nif git.merge.target == \"main\" {\n    print \"Merged into main\"\n}\n```"),
        "git.files.staged" => Some("**git.files.staged**: Array<File>\n\nAll staged files.\n\n**Example:**\n```githook\nforeach file in git.files.staged matching \"*.rs\" {\n    block_if content matches \"panic!\"\n}\n```"),
        "git.files.all" => Some("**git.files.all**: Array<File>\n\nAll tracked files in repo.\n\n**Example:**\n```githook\nforeach file in git.files.all matching \"*.md\" {\n    warn_if file.size > 100000\n}\n```"),
        "git.files.modified" => Some("**git.files.modified**: Array<File>\n\nModified files.\n\n**Example:**\n```githook\nforeach file in git.files.modified {\n    print \"Modified: \" + file.name\n}\n```"),
        "git.files.added" => Some("**git.files.added**: Array<File>\n\nNewly added files.\n\n**Example:**\n```githook\nforeach file in git.files.added {\n    print \"New file: \" + file.name\n}\n```"),
        "git.files.deleted" => Some("**git.files.deleted**: Array<File>\n\nDeleted files.\n\n**Example:**\n```githook\nforeach file in git.files.deleted {\n    print \"Deleted: \" + file.name\n}\n```"),
        "git.files.unstaged" => Some("**git.files.unstaged**: Array<File>\n\nUnstaged changes.\n\n**Example:**\n```githook\nforeach file in git.files.unstaged {\n    warn \"Unstaged: \" + file.name\n}\n```"),
        _ => None,
    }
}

fn format_macro_body(body: &[githook_syntax::Statement]) -> String {
    let mut result = String::new();
    for stmt in body {
        let line = match stmt {
            githook_syntax::Statement::Run { command, .. } => format!("    run \"{}\"", command),
            githook_syntax::Statement::Block { message, .. } => format!("    block \"{}\"", message),
            githook_syntax::Statement::Warn { message, .. } => format!("    warn \"{}\"", message),
            githook_syntax::Statement::BlockIf { message, .. } => {
                format!("    block_if ... message \"{}\"", message.as_deref().unwrap_or(""))
            }
            githook_syntax::Statement::WarnIf { message, .. } => {
                format!("    warn_if ... message \"{}\"", message.as_deref().unwrap_or(""))
            }
            githook_syntax::Statement::MacroCall { name, namespace, .. } => {
                if let Some(ns) = namespace {
                    format!("    @{}:{}", ns, name)
                } else {
                    format!("    @{}", name)
                }
            }
            _ => "    ...".to_string(),
        };
        result.push_str(&line);
        result.push('\n');
    }
    result
}

fn get_method_hover(method: &str) -> Option<String> {
    if let Some(doc) = get_method_doc(method) {
        return Some(format!(
            "**{}**\n\n{}\n\n**Example:**\n```githook\n{}\n```",
            doc.name,
            doc.description,
            doc.example
        ));
    }
    None
}

fn get_property_hover(name: &str) -> Option<String> {
    if let Some(doc) = get_property_doc(name) {
        return Some(format!(
            "**{}**\n\n{}\n\n**Example:**\n```githook\n{}\n```",
            doc.name,
            doc.description,
            doc.example
        ));
    }
    None
}

fn get_property_chain_hover(word: &str) -> Option<String> {
    if let Some(doc) = get_property_doc(word) {
        return Some(format!(
            "**{}**\n\n{}\n\n**Example:**\n```githook\n{}\n```",
            doc.name,
            doc.description,
            doc.example
        ));
    }
    None
}
