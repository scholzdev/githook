use crate::document::DocumentState;
use githook_syntax::{LetValue, Statement, ast::Expression};
use tower_lsp::lsp_types::*;

pub fn get_inlay_hints(doc: &DocumentState, range: Range) -> Vec<InlayHint> {
    let mut hints = Vec::new();

    if let Some(statements) = &doc.ast {
        for statement in statements {
            collect_hints(statement, &doc.text, range, &mut hints);
        }
    }

    hints
}

fn collect_hints(stmt: &Statement, source: &str, range: Range, hints: &mut Vec<InlayHint>) {
    match stmt {
        Statement::Let { name, value, span } => {
            if let Some(inferred_type) = infer_let_type(value) {
                let position = offset_to_position(source, span.start);

                let lines: Vec<&str> = source.lines().collect();
                let line_idx = position.line as usize;

                if line_idx >= lines.len() {
                    return;
                }

                let line = lines[line_idx];

                if let Some(name_pos) = line.find(name) {
                    let hint_pos = name_pos + name.len();
                    let hint_position = Position {
                        line: line_idx as u32,
                        character: hint_pos as u32,
                    };

                    if hint_position.line < range.start.line || hint_position.line > range.end.line
                    {
                        return;
                    }

                    hints.push(InlayHint {
                        position: hint_position,
                        label: InlayHintLabel::String(format!(": {}", inferred_type)),
                        kind: Some(InlayHintKind::TYPE),
                        text_edits: None,
                        tooltip: None,
                        padding_left: None,
                        padding_right: Some(true),
                        data: None,
                    });
                }
            }
        }

        Statement::If {
            then_body,
            else_body,
            ..
        } => {
            for s in then_body {
                collect_hints(s, source, range, hints);
            }
            if let Some(else_stmts) = else_body {
                for s in else_stmts {
                    collect_hints(s, source, range, hints);
                }
            }
        }

        Statement::ForEach { body, .. } => {
            for s in body {
                collect_hints(s, source, range, hints);
            }
        }

        Statement::Match { arms, .. } => {
            for arm in arms {
                for s in &arm.body {
                    collect_hints(s, source, range, hints);
                }
            }
        }

        Statement::Try {
            body, catch_body, ..
        } => {
            for s in body {
                collect_hints(s, source, range, hints);
            }
            for s in catch_body {
                collect_hints(s, source, range, hints);
            }
        }

        Statement::Group { body, .. } => {
            for s in body {
                collect_hints(s, source, range, hints);
            }
        }

        Statement::MacroDef { body, .. } => {
            for s in body {
                collect_hints(s, source, range, hints);
            }
        }

        _ => {}
    }
}

fn offset_to_position(source: &str, offset: usize) -> Position {
    let mut line = 0;
    let mut character = 0;

    for (idx, ch) in source.chars().enumerate() {
        if idx >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += 1;
        }
    }

    Position {
        line,
        character: character as u32,
    }
}

fn infer_let_type(value: &LetValue) -> Option<String> {
    match value {
        LetValue::String(_) => Some("String".to_string()),
        LetValue::Number(_) => Some("Number".to_string()),
        LetValue::Array(_) => Some("Array<String>".to_string()),
        LetValue::Expression(expr) => infer_expr_type(expr),
    }
}

fn infer_expr_type(expr: &Expression) -> Option<String> {
    match expr {
        Expression::String(_, _) => Some("String".to_string()),
        Expression::Number(_, _) => Some("Number".to_string()),
        Expression::Bool(_, _) => Some("Bool".to_string()),
        Expression::Array(_, _) => Some("Array".to_string()),
        Expression::Null(_) => Some("Null".to_string()),

        Expression::PropertyAccess {
            receiver, property, ..
        } => {
            if let Expression::Identifier(name, _) = receiver.as_ref() {
                if name == "git" {
                    match property.as_str() {
                        "branch" => Some("BranchInfo".to_string()),
                        "author" => Some("AuthorInfo".to_string()),
                        "commit" => Some("CommitInfo".to_string()),
                        "files" => Some("FilesCollection".to_string()),
                        "staged" | "all" | "modified" | "added" | "deleted" | "unstaged" => {
                            Some("Array<File>".to_string())
                        }
                        "diff" => Some("DiffCollection".to_string()),
                        "added_lines" | "removed_lines" => Some("Array<String>".to_string()),
                        _ => Some("String".to_string()),
                    }
                } else {
                    Some("String".to_string())
                }
            } else {
                Some("String".to_string())
            }
        }

        Expression::MethodCall {
            receiver, method, ..
        } => match method.as_str() {
            "upper" | "lower" | "trim" | "reverse" | "replace" => Some("String".to_string()),
            "split" | "lines" => Some("Array<String>".to_string()),
            "contains" | "starts_with" | "ends_with" => Some("Bool".to_string()),
            "filter" | "map" => infer_expr_type(receiver),
            "first" | "last" => Some("String".to_string()),
            "length" | "len" => Some("Number".to_string()),
            _ => None,
        },

        _ => None,
    }
}
