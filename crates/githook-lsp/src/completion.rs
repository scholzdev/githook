use crate::document::DocumentState;
use githook_syntax::ast::{Expression, LetValue, Statement};
use tower_lsp::lsp_types::*;

pub fn get_completions(doc: &DocumentState, position: Position) -> Vec<CompletionItem> {
    let mut completions = Vec::new();

    if let Some(context) = get_context(doc, position) {
        match context {
            GithookCompletionContext::MacroCall(namespace) => {
                if let Some(ast) = &doc.ast {
                    for stmt in ast {
                        if let githook_syntax::ast::Statement::MacroDef { name, params, .. } = stmt
                        {
                            let params_str = if params.is_empty() {
                                String::new()
                            } else {
                                format!("({})", params.join(", "))
                            };

                            completions.push(CompletionItem {
                                label: name.clone(),
                                kind: Some(CompletionItemKind::FUNCTION),
                                detail: Some(format!("macro{}", params_str)),
                                insert_text: Some(name.clone()),
                                ..Default::default()
                            });
                        }
                    }
                }

                if namespace.is_none() {
                    for (pkg, desc) in &[
                        ("git", "Git utilities package"),
                        ("preview/git", "Preview git package"),
                    ] {
                        completions.push(CompletionItem {
                            label: pkg.to_string(),
                            kind: Some(CompletionItemKind::MODULE),
                            detail: Some(desc.to_string()),
                            ..Default::default()
                        });
                    }
                }

                return completions;
            }
            GithookCompletionContext::PropertyAccess(prefix) => {
                return get_property_completions(&prefix);
            }
            GithookCompletionContext::ImportPath => {
                completions.push(CompletionItem {
                    label: "./helpers.ghook".to_string(),
                    kind: Some(CompletionItemKind::FILE),
                    detail: Some("Import local file".to_string()),
                    ..Default::default()
                });
                completions.push(CompletionItem {
                    label: "./macros.ghook".to_string(),
                    kind: Some(CompletionItemKind::FILE),
                    detail: Some("Import local file".to_string()),
                    ..Default::default()
                });
                return completions;
            }
            GithookCompletionContext::UsePath => {
                for (pkg, desc) in &[
                    ("@preview/git", "Git utilities package"),
                    ("@preview/security", "Security checks package"),
                    ("@preview/quality", "Code quality package"),
                ] {
                    completions.push(CompletionItem {
                        label: pkg.to_string(),
                        kind: Some(CompletionItemKind::MODULE),
                        detail: Some(desc.to_string()),
                        ..Default::default()
                    });
                }
                return completions;
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
        ("foreach", "Loop over collection"),
        ("break", "Exit loop"),
        ("continue", "Next iteration"),
        ("let", "Variable declaration"),
        ("match", "Pattern matching"),
        ("macro", "Define macro"),
        ("import", "Import local file"),
        ("use", "Use remote package"),
        ("print", "Print message"),
        ("parallel", "Run commands in parallel"),
        ("allow", "Allow specific command"),
    ] {
        completions.push(CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some(detail.to_string()),
            ..Default::default()
        });
    }

    for label in &["git", "true", "false", "null"] {
        let detail = match *label {
            "git" => "Git - Git operations and repository info",
            "true" => "bool - Boolean true value",
            "false" => "bool - Boolean false value",
            "null" => "null - Null value",
            _ => "",
        };

        completions.push(CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::CONSTANT),
            detail: Some(detail.to_string()),
            ..Default::default()
        });
    }

    for (label, detail) in &[
        ("http", "Http - HTTP client for making requests"),
        ("env", "Env - Environment variables"),
        ("file", "File - File system operations"),
        ("dir", "Dir - Directory operations"),
        ("glob", "Glob - Pattern matching"),
        ("exec", "Exec - Execute commands"),
    ] {
        completions.push(CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some(detail.to_string()),
            ..Default::default()
        });
    }

    completions
}

#[derive(Debug, PartialEq)]
enum GithookCompletionContext {
    Normal,
    MacroCall(Option<String>),
    PropertyAccess(String),
    ImportPath,
    UsePath,
}

fn infer_variable_type(
    doc: &DocumentState,
    var_name: &str,
    _current_line: usize,
) -> Option<String> {
    let ast = doc.ast.as_ref()?;

    for stmt in ast {
        if let Statement::Let { name, value, .. } = stmt
            && name == var_name
        {
            return infer_let_value_type(value);
        }
    }

    None
}

fn infer_let_value_type(value: &LetValue) -> Option<String> {
    match value {
        LetValue::String(_) => Some("string".to_string()),
        LetValue::Number(_) => Some("number".to_string()),
        LetValue::Array(_) => Some("array".to_string()),
        LetValue::Expression(expr) => infer_expression_type(expr),
    }
}

fn infer_expression_type(expr: &Expression) -> Option<String> {
    match expr {
        Expression::String(_, _) => Some("string".to_string()),
        Expression::Number(_, _) => Some("number".to_string()),
        Expression::Bool(_, _) => Some("bool".to_string()),
        Expression::Array(_, _) => Some("array".to_string()),
        Expression::PropertyAccess {
            receiver, property, ..
        } => infer_property_access_type(receiver, property),
        Expression::MethodCall {
            receiver, method, ..
        } => {
            if let Some(return_type) = infer_method_return_type(method) {
                Some(return_type)
            } else {
                infer_expression_type(receiver)
            }
        }
        Expression::Identifier(name, _) => match name.as_str() {
            "git" => Some("git".to_string()),
            "http" => Some("http".to_string()),
            "env" => Some("env".to_string()),
            _ => None,
        },
        _ => None,
    }
}

fn infer_property_access_type(receiver: &Expression, property: &str) -> Option<String> {
    let receiver_type = infer_expression_type(receiver)?;

    match (receiver_type.as_str(), property) {
        ("git", "files") => Some("files".to_string()),
        ("git", "diff") => Some("diff".to_string()),
        ("git", "branch") => Some("branch".to_string()),
        ("git", "commit") => Some("commit".to_string()),
        ("git", "author") => Some("author".to_string()),
        ("git", "remote") => Some("remote".to_string()),
        ("git", "stats") => Some("stats".to_string()),
        ("git", "merge") => Some("merge".to_string()),

        ("files", "all" | "staged" | "modified" | "added" | "deleted" | "unstaged") => {
            Some("array".to_string())
        }

        ("array", _) if property == "first" || property == "last" => Some("file".to_string()),

        _ => Some("string".to_string()),
    }
}

fn infer_method_return_type(method: &str) -> Option<String> {
    match method {
        "upper" | "lower" | "trim" | "replace" | "slice" => Some("string".to_string()),
        "split" | "lines" => Some("array".to_string()),
        "len" | "count" => Some("number".to_string()),
        "contains" | "starts_with" | "ends_with" => Some("bool".to_string()),
        _ => None,
    }
}

fn get_context(doc: &DocumentState, position: Position) -> Option<GithookCompletionContext> {
    let line_idx = position.line as usize;
    let lines: Vec<&str> = doc.text.lines().collect();

    if line_idx >= lines.len() {
        return Some(GithookCompletionContext::Normal);
    }

    let line = lines[line_idx];
    let char_idx = position.character as usize;

    if char_idx == 0 {
        return Some(GithookCompletionContext::Normal);
    }

    let before_cursor: String = line.chars().take(char_idx).collect();

    if let Some(dot_pos) = before_cursor.rfind('.') {
        let before_dot = &before_cursor[..dot_pos];

        if before_dot.trim_end().ends_with('"') || before_dot.trim_end().ends_with('}') {
            return Some(GithookCompletionContext::PropertyAccess(
                "string".to_string(),
            ));
        }

        let ident_start = before_dot
            .rfind(|c: char| c.is_whitespace() || "({[,=!<>".contains(c))
            .map(|pos| pos + 1)
            .unwrap_or(0);

        let prefix = &before_dot[ident_start..].trim();

        if !prefix.is_empty() {
            if let Some(var_type) = infer_variable_type(doc, prefix, line_idx) {
                return Some(GithookCompletionContext::PropertyAccess(var_type));
            }

            return Some(GithookCompletionContext::PropertyAccess(prefix.to_string()));
        }
    }

    if let Some(at_pos) = before_cursor.rfind('@') {
        let after_at = &before_cursor[at_pos + 1..];
        if let Some(dot_pos) = after_at.find('.') {
            let namespace = &after_at[..dot_pos];
            return Some(GithookCompletionContext::MacroCall(Some(
                namespace.to_string(),
            )));
        } else if after_at
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '/' || c == '-')
        {
            return Some(GithookCompletionContext::MacroCall(None));
        }
    }

    if before_cursor.contains("import") && before_cursor.trim_end().ends_with('"') {
        return Some(GithookCompletionContext::ImportPath);
    }

    if before_cursor.contains("use") && before_cursor.contains("\"@") {
        return Some(GithookCompletionContext::UsePath);
    }

    Some(GithookCompletionContext::Normal)
}

fn get_property_completions(prefix: &str) -> Vec<CompletionItem> {
    let mut completions = Vec::new();

    let properties: &[(&str, &str)] = match prefix {
        "git" => &[
            ("files", "FilesCollection - File collections"),
            ("diff", "DiffCollection - Diff lines"),
            ("branch", "BranchInfo - Current branch"),
            ("commit", "CommitInfo - Commit info"),
            ("author", "AuthorInfo - Author details"),
            ("remote", "RemoteInfo - Remote repository"),
            ("stats", "DiffStats - Diff statistics"),
            ("is_merge_commit", "bool - Is merge commit"),
            ("has_conflicts", "bool - Has conflicts"),
        ],
        "git.files" | "files" => &[
            ("staged", "Array<File> - Staged files"),
            ("all", "Array<File> - All tracked files"),
            ("modified", "Array<File> - Modified files"),
            ("added", "Array<File> - Added files"),
            ("deleted", "Array<File> - Deleted files"),
            ("unstaged", "Array<File> - Unstaged files"),
        ],
        "git.diff" | "diff" => &[
            ("added_lines", "Array<String> - Added lines"),
            ("removed_lines", "Array<String> - Removed lines"),
        ],
        "git.merge" | "merge" => &[
            ("source", "String - Source branch/commit of the merge"),
            ("target", "String - Target branch of the merge"),
        ],
        "git.branch" | "branch" => &[
            ("name", "String - Branch name"),
            ("is_main", "bool - Is main/master branch"),
        ],
        "git.commit" | "commit" => &[
            ("message", "String - Commit message"),
            ("hash", "String - Commit SHA hash"),
        ],
        "git.author" | "author" => &[
            ("name", "String - Author name"),
            ("email", "String - Author email"),
        ],
        "git.remote" | "remote" => &[
            ("name", "String - Remote name (e.g. origin)"),
            ("url", "String - Remote URL"),
        ],
        "git.stats" | "stats" => &[
            ("files_changed", "Number - Files changed count"),
            ("additions", "Number - Lines added"),
            ("deletions", "Number - Lines deleted"),
            ("modified_lines", "Number - Total lines modified"),
        ],
        "file" | "f" => &[
            ("name", "String - Filename"),
            ("path", "PathContext - Path object"),
            ("basename", "String - Base name without extension"),
            ("extension", "String - File extension"),
            ("dirname", "String - Directory path"),
            ("content", "String - File content"),
            ("diff", "String - Staged diff for file"),
            ("size", "Number - File size in bytes"),
            ("exists()", "bool - Check if file exists"),
            ("is_file()", "bool - Is regular file"),
            ("is_dir()", "bool - Is directory"),
            ("is_readable()", "bool - Is readable"),
            ("is_executable()", "bool - Is executable"),
            ("is_symlink()", "bool - Is symbolic link"),
            ("is_absolute()", "bool - Is absolute path"),
            ("is_relative()", "bool - Is relative path"),
            ("is_hidden()", "bool - Is hidden file"),
            ("modified_time()", "Number - Last modified timestamp"),
            ("created_time()", "Number - Creation timestamp"),
            ("permissions()", "Number - File permissions"),
            ("contains(pattern)", "bool - Contains text pattern"),
            ("starts_with(prefix)", "bool - Path starts with"),
            ("ends_with(suffix)", "bool - Path ends with"),
        ],
        "array" => &[
            ("length", "Number - Array length"),
            ("first()", "Value - First element"),
            ("last()", "Value - Last element"),
            ("is_empty()", "bool - Is array empty"),
            ("sum()", "Number - Sum of numeric elements"),
            ("filter(fn)", "Array - Filter elements"),
            ("map(fn)", "Array - Map elements"),
            ("find(fn)", "Value - Find first matching"),
            ("any(fn)", "bool - Any element matches"),
            ("all(fn)", "bool - All elements match"),
        ],
        p if p.starts_with("f") || p.starts_with("file") => &[
            ("name", "String - Filename"),
            ("path", "PathContext - Path object"),
            ("basename", "String - Base name without extension"),
            ("extension", "String - File extension"),
            ("dirname", "String - Directory path"),
            ("size", "Number - File size in bytes"),
            ("exists()", "bool - Check if file exists"),
            ("is_file()", "bool - Is regular file"),
            ("is_dir()", "bool - Is directory"),
            ("is_readable()", "bool - Is readable"),
            ("is_executable()", "bool - Is executable"),
            ("is_symlink()", "bool - Is symbolic link"),
            ("is_absolute()", "bool - Is absolute path"),
            ("is_relative()", "bool - Is relative path"),
            ("is_hidden()", "bool - Is hidden file"),
            ("modified_time()", "Number - Last modified timestamp"),
            ("created_time()", "Number - Creation timestamp"),
            ("permissions()", "Number - File permissions"),
            ("contains(pattern)", "bool - Contains text pattern"),
            ("starts_with(prefix)", "bool - Path starts with"),
            ("ends_with(suffix)", "bool - Path ends with"),
        ],
        p if p.ends_with(".path") || p == "path" => &[
            ("string", "String - Path as string"),
            ("basename", "String - Base name"),
            ("extension", "String - Extension"),
            ("parent", "String - Parent directory"),
            ("filename", "String - Filename"),
            ("join(suffix)", "String - Join path segments"),
        ],
        p if p.contains("response") || p == "resp" || p == "gh" || p == "r" => &[
            ("status", "Number - HTTP status code (e.g. 200, 404)"),
            ("ok", "bool - Whether status is 2xx success"),
            ("body", "String - Response body as text"),
            ("header(name)", "String - Get response header by name"),
            ("json()", "Object - Parse body as JSON"),
        ],
        "string" => {
            return get_string_method_completions();
        }
        _ => {
            if prefix.contains("name")
                || prefix.contains("message")
                || prefix.contains("email")
                || prefix.contains("url")
            {
                return get_string_method_completions();
            }
            if prefix.contains("size") || prefix.contains("count") || prefix.contains("length") {
                return get_number_method_completions();
            }
            if prefix.contains("files") || prefix.contains("array") {
                return get_array_method_completions();
            }
            &[]
        }
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

fn get_string_method_completions() -> Vec<CompletionItem> {
    vec![
        CompletionItem {
            label: "length".to_string(),
            kind: Some(CompletionItemKind::PROPERTY),
            detail: Some("Number - String length".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "upper()".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("String - Convert to uppercase".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "lower()".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("String - Convert to lowercase".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "trim()".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("String - Remove whitespace".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "reverse()".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("String - Reverse string".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "split(delimiter)".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("Array<String> - Split by delimiter".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "replace(from, to)".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("String - Replace substring".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "contains(needle)".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("bool - Check if contains".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "starts_with(prefix)".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("bool - Check if starts with".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "ends_with(suffix)".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("bool - Check if ends with".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "matches(pattern)".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("bool - Regex match".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "lines()".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("Array<String> - Split into lines".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "slice(start, end)".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("String - Substring from start to end (exclusive)".to_string()),
            ..Default::default()
        },
    ]
}

fn get_number_method_completions() -> Vec<CompletionItem> {
    vec![
        CompletionItem {
            label: "abs()".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("Number - Absolute value".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "floor()".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("Number - Round down".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "ceil()".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("Number - Round up".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "round()".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("Number - Round to nearest".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "sqrt()".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("Number - Square root".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "pow(exponent)".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("Number - Power of".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "sin()".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("Number - Sine".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "cos()".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("Number - Cosine".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "tan()".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("Number - Tangent".to_string()),
            ..Default::default()
        },
    ]
}

fn get_array_method_completions() -> Vec<CompletionItem> {
    vec![
        CompletionItem {
            label: "length".to_string(),
            kind: Some(CompletionItemKind::PROPERTY),
            detail: Some("Number - Array length".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "sum()".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("Number - Sum of all numbers".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "filter(fn)".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("Array - Filter with closure".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "map(fn)".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("Array - Transform with closure".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "find(fn)".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("Any - Find first match".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "any(fn)".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("bool - Check if any match".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "all(fn)".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("bool - Check if all match".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "first()".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("Any - First element".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "last()".to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some("Any - Last element".to_string()),
            ..Default::default()
        },
    ]
}
