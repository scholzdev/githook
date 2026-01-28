use tower_lsp::lsp_types::*;
use githook_syntax::Statement;

/// Get folding ranges for code blocks
pub fn get_folding_ranges(ast: &Option<Vec<Statement>>) -> Vec<FoldingRange> {
    let mut ranges = Vec::new();
    
    if let Some(statements) = ast {
        for stmt in statements {
            collect_folding_ranges(stmt, &mut ranges);
        }
    }
    
    ranges
}

fn collect_folding_ranges(stmt: &Statement, ranges: &mut Vec<FoldingRange>) {
    match stmt {
        Statement::MacroDefinition { span, body, .. } => {
            if !body.is_empty() {
                // Find the end line by getting the max span from body
                let end_line = body.iter()
                    .filter_map(|s| get_statement_span(s))
                    .map(|s| s.line)
                    .max()
                    .unwrap_or(span.line);
                
                ranges.push(FoldingRange {
                    start_line: (span.line - 1) as u32,
                    start_character: None,
                    end_line: (end_line - 1) as u32,
                    end_character: None,
                    kind: Some(FoldingRangeKind::Region),
                    collapsed_text: None,
                });
            }
            
            // Recursively fold inner blocks
            for inner_stmt in body {
                collect_folding_ranges(inner_stmt, ranges);
            }
        }
        Statement::When { span, body, else_body, .. } => {
            if !body.is_empty() {
                let end_line = body.iter()
                    .filter_map(|s| get_statement_span(s))
                    .map(|s| s.line)
                    .max()
                    .unwrap_or(span.line);
                
                ranges.push(FoldingRange {
                    start_line: (span.line - 1) as u32,
                    start_character: None,
                    end_line: (end_line - 1) as u32,
                    end_character: None,
                    kind: Some(FoldingRangeKind::Region),
                    collapsed_text: None,
                });
            }
            
            for inner_stmt in body {
                collect_folding_ranges(inner_stmt, ranges);
            }
            
            if let Some(else_stmts) = else_body {
                for inner_stmt in else_stmts {
                    collect_folding_ranges(inner_stmt, ranges);
                }
            }
        }
        Statement::Group { span, definition, .. } => {
            if !definition.body.is_empty() {
                let end_line = definition.body.iter()
                    .filter_map(|s| get_statement_span(s))
                    .map(|s| s.line)
                    .max()
                    .unwrap_or(span.line);
                
                ranges.push(FoldingRange {
                    start_line: (span.line - 1) as u32,
                    start_character: None,
                    end_line: (end_line - 1) as u32,
                    end_character: None,
                    kind: Some(FoldingRangeKind::Region),
                    collapsed_text: None,
                });
            }
        }
        _ => {}
    }
}

fn get_statement_span(stmt: &Statement) -> Option<githook_syntax::error::Span> {
    match stmt {
        Statement::Run(_, span) => Some(*span),
        Statement::Block(_, span) => Some(*span),
        Statement::MacroDefinition { span, .. } => Some(*span),
        Statement::MacroCall { span, .. } => Some(*span),
        Statement::When { span, .. } => Some(*span),
        Statement::Group { span, .. } => Some(*span),
        Statement::Import { span, .. } => Some(*span),
        _ => None,
    }
}
