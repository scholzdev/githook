use tower_lsp::lsp_types::*;
use githook_syntax::Statement;

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
        Statement::MacroDef { span, body, .. } => {
            if !body.is_empty() {
                let end_line = body.iter()
                    .filter_map(get_statement_span)
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
        }
        Statement::If { span, then_body, else_body, .. } => {
            if !then_body.is_empty() {
                let end_line = then_body.iter()
                    .filter_map(get_statement_span)
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
            
            for inner_stmt in then_body {
                collect_folding_ranges(inner_stmt, ranges);
            }
            
            if let Some(else_stmts) = else_body {
                for inner_stmt in else_stmts {
                    collect_folding_ranges(inner_stmt, ranges);
                }
            }
        }
        Statement::ForEach { body, span, .. } => {
            if !body.is_empty() {
                let end_line = body.iter()
                    .filter_map(get_statement_span)
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
    use githook_syntax::ast::Statement::*;
    match stmt {
        Run { span, .. } => Some(*span),
        Block { span, .. } => Some(*span),
        Warn { span, .. } => Some(*span),
        MacroDef { span, .. } => Some(*span),
        MacroCall { span, .. } => Some(*span),
        If { span, .. } => Some(*span),
        ForEach { span, .. } => Some(*span),
        Import { span, .. } => Some(*span),
        _ => None,
    }
}
