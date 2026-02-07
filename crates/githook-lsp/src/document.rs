use githook_syntax::{ast::Statement, error::ParseError, lexer, parser};
use tower_lsp::lsp_types::Diagnostic;

type ParseResult = (
    Option<Vec<Statement>>,
    Vec<(String, Option<githook_syntax::error::Span>)>,
);

pub struct DocumentState {
    pub text: String,
    pub ast: Option<Vec<Statement>>,
    pub errors: Vec<(String, Option<githook_syntax::error::Span>)>,
}

impl DocumentState {
    pub fn new(text: String, _current_uri: Option<&str>) -> Self {
        let (ast, errors) = Self::parse(&text);
        Self { text, ast, errors }
    }

    fn parse(text: &str) -> ParseResult {
        match lexer::tokenize(text) {
            Ok(tokens) => match parser::parse(tokens) {
                Ok(ast) => (Some(ast), vec![]),
                Err(e) => {
                    let span = e.downcast_ref::<ParseError>().and_then(|pe| pe.span());
                    let error_msg = e.to_string();
                    (None, vec![(error_msg, span)])
                }
            },
            Err(lex_error) => {
                let span = Some(lex_error.span());
                (None, vec![(lex_error.to_string(), span)])
            }
        }
    }

    pub fn diagnostics(&self) -> Option<Vec<Diagnostic>> {
        if self.errors.is_empty() {
            return None;
        }

        let diagnostics = self
            .errors
            .iter()
            .map(|(error, span)| {
                crate::diagnostics::parse_error_to_diagnostic(error, span, &self.text)
            })
            .collect();

        Some(diagnostics)
    }
}
