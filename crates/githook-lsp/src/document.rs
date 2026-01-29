use githook_syntax::{lexer, parser, ast::Statement};
use tower_lsp::lsp_types::Diagnostic;

/// Represents the state of a document in the LSP
pub struct DocumentState {
    /// Raw text content
    pub text: String,
    /// Parsed AST (if successful)
    pub ast: Option<Vec<Statement>>,
    /// Parse errors
    pub errors: Vec<String>,
}

impl DocumentState {
    pub fn new(text: String, _current_uri: Option<&str>) -> Self {
        let (ast, errors) = Self::parse(&text);
        Self { text, ast, errors }
    }
    
    fn parse(text: &str) -> (Option<Vec<Statement>>, Vec<String>) {
        match lexer::tokenize(text) {
            Ok(tokens) => match parser::parse(tokens) {
                Ok(ast) => (Some(ast), vec![]),
                Err(e) => (None, vec![e.to_string()]),
            },
            Err(lex_error) => {
                (None, vec![lex_error.to_string()])
            }
        }
    }

    /// Get LSP diagnostics from parse errors
    pub fn diagnostics(&self) -> Option<Vec<Diagnostic>> {
        if self.errors.is_empty() {
            return None;
        }

        let diagnostics = self.errors.iter().map(|error| {
            crate::diagnostics::parse_error_to_diagnostic(error, &self.text)
        }).collect();

        Some(diagnostics)
    }
}
