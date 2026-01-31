use githook_syntax::{lexer, parser, ast::Statement};
use tower_lsp::lsp_types::Diagnostic;

/// Represents the state of a document in the LSP
pub struct DocumentState {
    /// Raw text content
    pub text: String,
    /// Parsed AST (if successful)
    pub ast: Option<Vec<Statement>>,
    /// Parse errors (error message, optional span)
    pub errors: Vec<(String, Option<githook_syntax::error::Span>)>,
}

impl DocumentState {
    pub fn new(text: String, _current_uri: Option<&str>) -> Self {
        let (ast, errors) = Self::parse(&text);
        Self { text, ast, errors }
    }
    
    fn parse(text: &str) -> (Option<Vec<Statement>>, Vec<(String, Option<githook_syntax::error::Span>)>) {
        match lexer::tokenize(text) {
            Ok(tokens) => match parser::parse(tokens) {
                Ok(ast) => (Some(ast), vec![]),
                Err(e) => {
                    // Try to extract span from anyhow error
                    let error_msg = e.to_string();
                    let span = extract_span_from_error(&error_msg);
                    (None, vec![(error_msg, span)])
                },
            },
            Err(lex_error) => {
                let span = Some(lex_error.span());
                (None, vec![(lex_error.to_string(), span)])
            }
        }
    }

    /// Get LSP diagnostics from parse errors
    pub fn diagnostics(&self) -> Option<Vec<Diagnostic>> {
        if self.errors.is_empty() {
            return None;
        }

        let diagnostics = self.errors.iter().map(|(error, span)| {
            crate::diagnostics::parse_error_to_diagnostic(error, span, &self.text)
        }).collect();

        Some(diagnostics)
    }
}

/// Extract span from error message like "Expected string after 'print', got Some(SpannedToken { token: Identifier(\"git\"), span: Span { line: 6, col: 7, start: 47, end: 50 } })"
fn extract_span_from_error(error: &str) -> Option<githook_syntax::error::Span> {
    // Look for "span: Span { line: X, col: Y, start: Z, end: W }"
    if let Some(span_start) = error.find("span: Span { line:") {
        let span_part = &error[span_start..];
        
        // Extract line
        let line = if let Some(line_start) = span_part.find("line:") {
            let line_str = &span_part[line_start + 5..];
            line_str.split(',').next()?.trim().parse::<usize>().ok()?
        } else {
            return None;
        };
        
        // Extract col
        let col = if let Some(col_start) = span_part.find("col:") {
            let col_str = &span_part[col_start + 4..];
            col_str.split(',').next()?.trim().parse::<usize>().ok()?
        } else {
            return None;
        };
        
        // Extract start
        let start = if let Some(start_idx) = span_part.find("start:") {
            let start_str = &span_part[start_idx + 6..];
            start_str.split(',').next()?.trim().parse::<usize>().ok()?
        } else {
            return None;
        };
        
        // Extract end
        let end = if let Some(end_idx) = span_part.find("end:") {
            let end_str = &span_part[end_idx + 4..];
            end_str.split('}').next()?.trim().parse::<usize>().ok()?
        } else {
            return None;
        };
        
        Some(githook_syntax::error::Span::new(line, col, start, end))
    } else {
        None
    }
}
