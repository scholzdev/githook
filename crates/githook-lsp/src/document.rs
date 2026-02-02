use githook_syntax::{ast::Statement, lexer, parser};
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
                    let error_msg = e.to_string();
                    let span = extract_span_from_error(&error_msg);
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

fn extract_span_from_error(error: &str) -> Option<githook_syntax::error::Span> {
    if let Some(at_pos) = error.find(" at line ") {
        let after_at = &error[at_pos + 9..];

        let line = if let Some(comma_pos) = after_at.find(',') {
            after_at[..comma_pos].trim().parse::<usize>().ok()?
        } else {
            return None;
        };

        let col = if let Some(col_start) = after_at.find("column ") {
            let col_str = &after_at[col_start + 7..];
            let col_digits: String = col_str.chars().take_while(|c| c.is_ascii_digit()).collect();
            col_digits.parse::<usize>().ok()?
        } else {
            return None;
        };

        Some(githook_syntax::error::Span::new(line, col, 0, 1))
    } else if let Some(span_start) = error.find("span: Span { line:") {
        let span_part = &error[span_start..];

        let line = if let Some(line_start) = span_part.find("line:") {
                let line_str = &span_part[line_start + 5..];
                line_str.split(',').next()?.trim().parse::<usize>().ok()?
            } else {
                return None;
            };

            let col = if let Some(col_start) = span_part.find("col:") {
                let col_str = &span_part[col_start + 4..];
                col_str.split(',').next()?.trim().parse::<usize>().ok()?
            } else {
                return None;
            };

            let start = if let Some(start_idx) = span_part.find("start:") {
                let start_str = &span_part[start_idx + 6..];
                start_str.split(',').next()?.trim().parse::<usize>().ok()?
            } else {
                return None;
            };

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
