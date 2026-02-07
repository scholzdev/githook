mod expressions;
mod statements;

use crate::ast::*;
use crate::error::Span;
use crate::lexer::{SpannedToken, Token};
use anyhow::{Result, bail};

/// Recursive-descent parser for the Githook scripting language.
///
/// Consumes a sequence of [`SpannedToken`]s and produces a `Vec<`[`Statement`]`>`.
/// Use the free function [`parse()`] for a convenient entry point.
pub struct Parser {
    pub(super) tokens: Vec<SpannedToken>,
    pub(super) pos: usize,
}

impl Parser {
    /// Creates a new parser from a token stream.
    pub fn new(tokens: Vec<SpannedToken>) -> Self {
        Self { tokens, pos: 0 }
    }

    #[inline]
    pub(super) fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|st| &st.token)
    }

    #[inline]
    pub(super) fn peek_nth(&self, n: usize) -> Option<&Token> {
        self.tokens.get(self.pos + n).map(|st| &st.token)
    }

    #[inline]
    pub(super) fn peek_span(&self) -> Option<Span> {
        self.tokens.get(self.pos).map(|st| st.span)
    }

    #[inline]
    pub(super) fn advance(&mut self) -> Option<SpannedToken> {
        if self.pos < self.tokens.len() {
            let token = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(token)
        } else {
            None
        }
    }

    pub(super) fn expect(&mut self, expected: Token) -> Result<Span> {
        match self.advance() {
            Some(st) if st.token == expected => Ok(st.span),
            Some(st) => {
                let expected_str = expected.display_name();
                let found_str = st.token.display_name();
                bail!(
                    "expected {}, got {} at line {}, column {}",
                    expected_str,
                    found_str,
                    st.span.line,
                    st.span.col
                )
            }
            None => {
                let expected_str = expected.display_name();
                bail!("expected {}, got end of file", expected_str)
            }
        }
    }

    #[inline]
    pub(super) fn skip_newlines(&mut self) {
        while matches!(self.peek(), Some(Token::Newline) | Some(Token::Comment(_))) {
            self.advance();
        }
    }

    pub(super) fn parse_body(&mut self) -> Result<Vec<Statement>> {
        let mut statements = Vec::with_capacity(16);

        self.skip_newlines();

        while !matches!(self.peek(), Some(Token::RightBrace) | None) {
            statements.push(self.parse_statement()?);
            self.skip_newlines();
        }

        Ok(statements)
    }
}

/// Parses a token stream into a list of [`Statement`] nodes.
///
/// This is the main entry point for the Githook parser. Errors are
/// returned as `anyhow::Error` wrapping a [`ParseError`](`crate::error::ParseError`).
pub fn parse(tokens: Vec<SpannedToken>) -> Result<Vec<Statement>> {
    let mut parser = Parser::new(tokens);
    let mut statements = Vec::with_capacity(32);

    parser.skip_newlines();

    while parser.peek().is_some() {
        statements.push(parser.parse_statement()?);
        parser.skip_newlines();
    }

    Ok(statements)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

    #[test]
    fn test_parse_print_statement() {
        let input = r#"print "hello""#;
        let tokens = tokenize(input).unwrap();
        let ast = parse(tokens).unwrap();

        assert_eq!(ast.len(), 1);
        assert!(matches!(ast[0], Statement::Print { .. }));
    }

    #[test]
    fn test_parse_let_statement() {
        let input = r#"let name = "John""#;
        let tokens = tokenize(input).unwrap();
        let ast = parse(tokens).unwrap();

        assert_eq!(ast.len(), 1);
        assert!(matches!(ast[0], Statement::Let { .. }));
    }

    #[test]
    fn test_parse_run_statement() {
        let input = r#"run "npm test""#;
        let tokens = tokenize(input).unwrap();
        let ast = parse(tokens).unwrap();

        assert_eq!(ast.len(), 1);
        assert!(matches!(ast[0], Statement::Run { .. }));
    }

    #[test]
    fn test_parse_if_statement() {
        let input = r#"if x == 1 { print "yes" }"#;
        let tokens = tokenize(input).unwrap();
        let ast = parse(tokens).unwrap();

        assert_eq!(ast.len(), 1);
        assert!(matches!(ast[0], Statement::If { .. }));
    }

    #[test]
    fn test_parse_if_else_statement() {
        let input = r#"if x == 1 { print "yes" } else { print "no" }"#;
        let tokens = tokenize(input).unwrap();
        let ast = parse(tokens).unwrap();

        assert_eq!(ast.len(), 1);
        if let Statement::If { else_body, .. } = &ast[0] {
            assert!(else_body.is_some());
        } else {
            panic!("Expected If statement");
        }
    }

    #[test]
    fn test_parse_group_statement() {
        let input = r#"group test info { print "test" }"#;
        let tokens = tokenize(input).unwrap();
        let ast = parse(tokens).unwrap();

        assert_eq!(ast.len(), 1);
        assert!(matches!(ast[0], Statement::Group { .. }));
    }

    #[test]
    fn test_parse_foreach_statement() {
        let input = r#"foreach git.files.staged matching "*.rs" { file in print file.name }"#;
        let tokens = tokenize(input).unwrap();
        let ast = parse(tokens).unwrap();

        assert_eq!(ast.len(), 1);
        assert!(matches!(ast[0], Statement::ForEach { .. }));
    }

    #[test]
    fn test_parse_macro_def() {
        let input = r#"macro check_files { print "checking" }"#;
        let tokens = tokenize(input).unwrap();
        let ast = parse(tokens).unwrap();

        assert_eq!(ast.len(), 1);
        assert!(matches!(ast[0], Statement::MacroDef { .. }));
    }

    #[test]
    fn test_parse_macro_call() {
        let input = r#"@check_files()"#;
        let tokens = tokenize(input).unwrap();
        let ast = parse(tokens).unwrap();

        assert_eq!(ast.len(), 1);
        assert!(matches!(ast[0], Statement::MacroCall { .. }));
    }

    #[test]
    fn test_parse_import_statement() {
        let input = r#"import "./helpers.ghook" as helpers"#;
        let tokens = tokenize(input).unwrap();
        let ast = parse(tokens).unwrap();

        assert_eq!(ast.len(), 1);
        assert!(matches!(ast[0], Statement::Import { .. }));
    }

    #[test]
    fn test_parse_binary_expression() {
        let input = r#"if x == 1 and y == 2 { print "ok" }"#;
        let tokens = tokenize(input).unwrap();
        let ast = parse(tokens).unwrap();

        assert_eq!(ast.len(), 1);
        if let Statement::If { condition, .. } = &ast[0] {
            assert!(matches!(condition, Expression::Binary { .. }));
        } else {
            panic!("Expected If statement");
        }
    }

    #[test]
    fn test_parse_property_access() {
        let input = r#"print git.files.staged"#;
        let tokens = tokenize(input).unwrap();
        let ast = parse(tokens).unwrap();

        assert_eq!(ast.len(), 1);
        if let Statement::Print { message, .. } = &ast[0] {
            assert!(matches!(message, Expression::PropertyAccess { .. }));
        } else {
            panic!("Expected Print statement");
        }
    }

    #[test]
    fn test_parse_method_call() {
        let input = r#"print files.length()"#;
        let tokens = tokenize(input).unwrap();
        let ast = parse(tokens).unwrap();

        assert_eq!(ast.len(), 1);
        if let Statement::Print { message, .. } = &ast[0] {
            assert!(matches!(message, Expression::MethodCall { .. }));
        } else {
            panic!("Expected Print statement");
        }
    }

    #[test]
    fn test_parse_array_literal() {
        let input = r#"let items = [1, 2, 3]"#;
        let tokens = tokenize(input).unwrap();
        let ast = parse(tokens).unwrap();

        assert_eq!(ast.len(), 1);
        if let Statement::Let { value, .. } = &ast[0] {
            assert!(matches!(
                value,
                LetValue::Expression(Expression::Array(_, _))
            ));
        } else {
            panic!("Expected Let statement");
        }
    }

    #[test]
    fn test_parse_multiple_statements() {
        let input = r#"
            print "hello"
            print "world"
            let x = 1
        "#;
        let tokens = tokenize(input).unwrap();
        let ast = parse(tokens).unwrap();

        assert_eq!(ast.len(), 3);
    }

    #[test]
    fn test_parse_error_missing_brace() {
        let input = r#"group test info"#;
        let tokens = tokenize(input).unwrap();
        let result = parse(tokens);

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_unexpected_token() {
        let input = r#"print print"#;
        let tokens = tokenize(input).unwrap();
        let result = parse(tokens);

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_nested_blocks() {
        let input = r#"
            if x == 1 {
                if y == 2 {
                    print "nested"
                }
            }
        "#;
        let tokens = tokenize(input).unwrap();
        let ast = parse(tokens).unwrap();

        assert_eq!(ast.len(), 1);
        if let Statement::If { then_body, .. } = &ast[0] {
            assert_eq!(then_body.len(), 1);
            assert!(matches!(then_body[0], Statement::If { .. }));
        } else {
            panic!("Expected If statement");
        }
    }

    // ── Ternary if-then-else expression parsing ───────────────

    #[test]
    fn test_parse_ternary_in_let() {
        let input = r#"let x = if true then "a" else "b""#;
        let tokens = tokenize(input).unwrap();
        let ast = parse(tokens).unwrap();

        assert_eq!(ast.len(), 1);
        if let Statement::Let { value, .. } = &ast[0] {
            assert!(matches!(
                value,
                LetValue::Expression(Expression::IfExpr { .. })
            ));
        } else {
            panic!("Expected Let statement");
        }
    }

    #[test]
    fn test_parse_ternary_in_print() {
        let input = r#"print if x > 0 then "pos" else "neg""#;
        let tokens = tokenize(input).unwrap();
        let ast = parse(tokens).unwrap();

        assert_eq!(ast.len(), 1);
        if let Statement::Print { message, .. } = &ast[0] {
            assert!(matches!(message, Expression::IfExpr { .. }));
        } else {
            panic!("Expected Print statement");
        }
    }

    #[test]
    fn test_parse_ternary_nested() {
        let input = r#"let x = if a then "x" else if b then "y" else "z""#;
        let tokens = tokenize(input).unwrap();
        let ast = parse(tokens).unwrap();

        assert_eq!(ast.len(), 1);
        if let Statement::Let {
            value: LetValue::Expression(Expression::IfExpr { else_expr, .. }),
            ..
        } = &ast[0]
        {
            assert!(matches!(else_expr.as_ref(), Expression::IfExpr { .. }));
        } else {
            panic!("Expected nested IfExpr");
        }
    }

    #[test]
    fn test_parse_ternary_missing_then() {
        let input = r#"let x = if true "a" else "b""#;
        let tokens = tokenize(input).unwrap();
        let result = parse(tokens);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_ternary_missing_else() {
        let input = r#"let x = if true then "a""#;
        let tokens = tokenize(input).unwrap();
        let result = parse(tokens);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_then_keyword_tokenizes() {
        let input = "then";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token, Token::Then);
    }

    // ── Index / bracket access parsing tests ─────────────────────────

    #[test]
    fn test_parse_index_access_string_key() {
        let input = r#"let x = data["key"]"#;
        let tokens = tokenize(input).unwrap();
        let ast = parse(tokens).unwrap();
        assert_eq!(ast.len(), 1);
        if let Statement::Let { value, .. } = &ast[0] {
            if let LetValue::Expression(Expression::IndexAccess {
                receiver, index, ..
            }) = value
            {
                assert!(
                    matches!(receiver.as_ref(), Expression::Identifier(name, _) if name == "data")
                );
                assert!(matches!(index.as_ref(), Expression::String(s, _) if s == "key"));
            } else {
                panic!("Expected IndexAccess expression, got {:?}", value);
            }
        } else {
            panic!("Expected Let statement");
        }
    }

    #[test]
    fn test_parse_index_access_number() {
        let input = r#"let x = arr[0]"#;
        let tokens = tokenize(input).unwrap();
        let ast = parse(tokens).unwrap();
        assert_eq!(ast.len(), 1);
        if let Statement::Let { value, .. } = &ast[0] {
            if let LetValue::Expression(Expression::IndexAccess {
                receiver, index, ..
            }) = value
            {
                assert!(
                    matches!(receiver.as_ref(), Expression::Identifier(name, _) if name == "arr")
                );
                assert!(matches!(index.as_ref(), Expression::Number(n, _) if *n == 0.0));
            } else {
                panic!("Expected IndexAccess expression, got {:?}", value);
            }
        } else {
            panic!("Expected Let statement");
        }
    }

    #[test]
    fn test_parse_chained_index_access() {
        let input = r#"let x = data[0]["name"]"#;
        let tokens = tokenize(input).unwrap();
        let ast = parse(tokens).unwrap();
        assert_eq!(ast.len(), 1);
        if let Statement::Let { value, .. } = &ast[0] {
            if let LetValue::Expression(Expression::IndexAccess {
                receiver, index, ..
            }) = value
            {
                // Outer: ..["name"]
                assert!(matches!(index.as_ref(), Expression::String(s, _) if s == "name"));
                // Inner: data[0]
                assert!(matches!(receiver.as_ref(), Expression::IndexAccess { .. }));
            } else {
                panic!("Expected IndexAccess expression, got {:?}", value);
            }
        } else {
            panic!("Expected Let statement");
        }
    }

    #[test]
    fn test_parse_dot_then_bracket() {
        let input = r#"let x = response.json["id"]"#;
        let tokens = tokenize(input).unwrap();
        let ast = parse(tokens).unwrap();
        assert_eq!(ast.len(), 1);
        if let Statement::Let { value, .. } = &ast[0] {
            if let LetValue::Expression(Expression::IndexAccess {
                receiver, index, ..
            }) = value
            {
                assert!(matches!(index.as_ref(), Expression::String(s, _) if s == "id"));
                assert!(
                    matches!(receiver.as_ref(), Expression::PropertyAccess { property, .. } if property == "json")
                );
            } else {
                panic!("Expected IndexAccess expression, got {:?}", value);
            }
        } else {
            panic!("Expected Let statement");
        }
    }

    #[test]
    fn test_parse_bracket_with_expression() {
        let input = r#"let x = arr[1 + 2]"#;
        let tokens = tokenize(input).unwrap();
        let ast = parse(tokens).unwrap();
        assert_eq!(ast.len(), 1);
        if let Statement::Let { value, .. } = &ast[0] {
            if let LetValue::Expression(Expression::IndexAccess { index, .. }) = value {
                assert!(matches!(index.as_ref(), Expression::Binary { .. }));
            } else {
                panic!("Expected IndexAccess expression, got {:?}", value);
            }
        } else {
            panic!("Expected Let statement");
        }
    }
}
