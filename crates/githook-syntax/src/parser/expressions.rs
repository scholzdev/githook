//! Expression parsing methods for the recursive-descent parser.

use super::Parser;
use crate::ast::*;
use crate::error::Span;
use crate::lexer::{SpannedToken, Token};
use anyhow::{Result, bail};

impl Parser {
    pub fn parse_expression(&mut self) -> Result<Expression> {
        self.parse_logical_or()
    }

    fn parse_logical_or(&mut self) -> Result<Expression> {
        let mut left = self.parse_logical_and()?;

        while matches!(self.peek(), Some(Token::Or)) {
            self.advance();
            let right = self.parse_logical_and()?;
            let span = left.span().merge(right.span());

            left = Expression::Binary {
                left: Box::new(left),
                op: BinaryOp::Or,
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn parse_logical_and(&mut self) -> Result<Expression> {
        let mut left = self.parse_comparison()?;

        while matches!(self.peek(), Some(Token::And)) {
            self.advance();
            let right = self.parse_comparison()?;
            let span = left.span().merge(right.span());

            left = Expression::Binary {
                left: Box::new(left),
                op: BinaryOp::And,
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expression> {
        let left = self.parse_additive()?;

        let op = match self.peek() {
            Some(Token::Eq) => {
                self.advance();
                BinaryOp::Eq
            }
            Some(Token::Ne) => {
                self.advance();
                BinaryOp::Ne
            }
            Some(Token::Lt) => {
                self.advance();
                BinaryOp::Lt
            }
            Some(Token::Le) => {
                self.advance();
                BinaryOp::Le
            }
            Some(Token::Gt) => {
                self.advance();
                BinaryOp::Gt
            }
            Some(Token::Ge) => {
                self.advance();
                BinaryOp::Ge
            }
            _ => return Ok(left),
        };

        let right = self.parse_additive()?;
        let span = left.span().merge(right.span());

        Ok(Expression::Binary {
            left: Box::new(left),
            op,
            right: Box::new(right),
            span,
        })
    }

    fn parse_additive(&mut self) -> Result<Expression> {
        let mut left = self.parse_multiplicative()?;

        while matches!(self.peek(), Some(Token::Plus) | Some(Token::Minus)) {
            let op = match self.peek() {
                Some(Token::Plus) => {
                    self.advance();
                    BinaryOp::Add
                }
                Some(Token::Minus) => {
                    self.advance();
                    BinaryOp::Sub
                }
                _ => unreachable!(),
            };

            let right = self.parse_multiplicative()?;
            let span = left.span().merge(right.span());

            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expression> {
        let mut left = self.parse_unary()?;

        while matches!(
            self.peek(),
            Some(Token::Star) | Some(Token::Slash) | Some(Token::Percent)
        ) {
            let op = match self.peek() {
                Some(Token::Star) => {
                    self.advance();
                    BinaryOp::Mul
                }
                Some(Token::Slash) => {
                    self.advance();
                    BinaryOp::Div
                }
                Some(Token::Percent) => {
                    self.advance();
                    BinaryOp::Mod
                }
                _ => unreachable!(),
            };

            let right = self.parse_unary()?;
            let span = left.span().merge(right.span());

            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expression> {
        if matches!(self.peek(), Some(Token::Not)) {
            let start_span = self.advance().expect("peek confirmed Not token").span;
            let expr = self.parse_unary()?;
            let span = start_span.merge(expr.span());

            return Ok(Expression::Unary {
                op: UnaryOp::Not,
                expr: Box::new(expr),
                span,
            });
        }

        if matches!(self.peek(), Some(Token::Minus)) {
            let start_span = self.advance().expect("peek confirmed Minus token").span;
            let expr = self.parse_unary()?;
            let span = start_span.merge(expr.span());

            return Ok(Expression::Unary {
                op: UnaryOp::Minus,
                expr: Box::new(expr),
                span,
            });
        }

        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expression> {
        let mut expr = self.parse_primary()?;

        while let Some(token) = self.peek() {
            match token {
                Token::Dot => {
                    self.advance();

                    let name = match self.advance() {
                        Some(SpannedToken {
                            token: Token::Identifier(id),
                            ..
                        }) => id,
                        other => bail!("Expected identifier after '.', got {:?}", other),
                    };

                    if matches!(self.peek(), Some(Token::LeftParen)) {
                        self.advance();

                        let mut args = Vec::new();

                        while !matches!(self.peek(), Some(Token::RightParen)) {
                            self.skip_newlines();
                            args.push(self.parse_expression()?);

                            if matches!(self.peek(), Some(Token::Comma)) {
                                self.advance();
                            } else {
                                break;
                            }
                        }

                        let end_span = self.expect(Token::RightParen)?;
                        let span = expr.span().merge(&end_span);

                        expr = Expression::MethodCall {
                            receiver: Box::new(expr),
                            method: name,
                            args,
                            span,
                        };
                    } else {
                        let end_span = self.peek_span().unwrap_or(*expr.span());
                        let span = expr.span().merge(&end_span);

                        expr = Expression::PropertyAccess {
                            receiver: Box::new(expr),
                            property: name,
                            span,
                        };
                    }
                }
                Token::LeftParen => {
                    if let Expression::Identifier(name, id_span) = expr {
                        self.advance();

                        let mut args = Vec::new();

                        while !matches!(self.peek(), Some(Token::RightParen)) {
                            self.skip_newlines();
                            args.push(self.parse_expression()?);

                            if matches!(self.peek(), Some(Token::Comma)) {
                                self.advance();
                            } else {
                                break;
                            }
                        }

                        let end_span = self.expect(Token::RightParen)?;
                        let span = id_span.merge(&end_span);

                        expr = Expression::MethodCall {
                            receiver: Box::new(Expression::Identifier(name.clone(), id_span)),
                            method: name,
                            args,
                            span,
                        };
                    } else {
                        break;
                    }
                }
                Token::LeftBracket => {
                    self.advance();
                    let index = self.parse_expression()?;
                    let end_span = self.expect(Token::RightBracket)?;
                    let span = expr.span().merge(&end_span);

                    expr = Expression::IndexAccess {
                        receiver: Box::new(expr),
                        index: Box::new(index),
                        span,
                    };
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expression> {
        match self.peek() {
            Some(Token::True) => {
                let span = self.advance().expect("peek confirmed True token").span;
                Ok(Expression::Bool(true, span))
            }

            Some(Token::False) => {
                let span = self.advance().expect("peek confirmed False token").span;
                Ok(Expression::Bool(false, span))
            }

            Some(Token::Null) => {
                let span = self.advance().expect("peek confirmed Null token").span;
                Ok(Expression::Null(span))
            }

            Some(Token::Number(_)) => {
                let st = self.advance().expect("peek confirmed Number token");
                if let Token::Number(n) = st.token {
                    Ok(Expression::Number(n, st.span))
                } else {
                    unreachable!()
                }
            }

            Some(Token::String(_)) => {
                let st = self.advance().expect("peek confirmed String token");
                if let Token::String(s) = st.token {
                    if s.contains("${") {
                        self.parse_interpolated_string(s, st.span)
                    } else {
                        Ok(Expression::String(s, st.span))
                    }
                } else {
                    unreachable!()
                }
            }

            Some(Token::Identifier(_)) => {
                let st = self.advance().expect("peek confirmed Identifier token");
                if let Token::Identifier(id) = st.token {
                    if matches!(self.peek(), Some(Token::FatArrow)) {
                        let start_span = st.span;
                        self.advance();
                        let body = Box::new(self.parse_expression()?);
                        let span = start_span.merge(body.span());
                        Ok(Expression::Closure {
                            param: id,
                            body,
                            span,
                        })
                    } else {
                        Ok(Expression::Identifier(id, st.span))
                    }
                } else {
                    unreachable!()
                }
            }

            Some(Token::LeftBracket) => {
                let start_span = self
                    .advance()
                    .expect("peek confirmed LeftBracket token")
                    .span;
                let mut items = Vec::with_capacity(8);

                self.skip_newlines();

                while !matches!(self.peek(), Some(Token::RightBracket)) {
                    items.push(self.parse_expression()?);

                    if matches!(self.peek(), Some(Token::Comma)) {
                        self.advance();
                        self.skip_newlines();
                    } else {
                        break;
                    }
                }

                let end_span = self.expect(Token::RightBracket)?;
                let span = start_span.merge(&end_span);

                Ok(Expression::Array(items, span))
            }

            Some(Token::LeftParen) => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(Token::RightParen)?;
                Ok(expr)
            }

            Some(Token::If) => {
                let start_span = self.advance().expect("peek confirmed If token").span;
                let condition = Box::new(self.parse_expression()?);
                self.expect(Token::Then)?;
                let then_expr = Box::new(self.parse_expression()?);
                self.expect(Token::Else)?;
                let else_expr = Box::new(self.parse_expression()?);
                let span = start_span.merge(else_expr.span());
                Ok(Expression::IfExpr {
                    condition,
                    then_expr,
                    else_expr,
                    span,
                })
            }

            other => {
                let token_str = other
                    .as_ref()
                    .map(|t| t.to_string())
                    .unwrap_or_else(|| "end of input".to_string());
                bail!("Unexpected token in expression: {}", token_str)
            }
        }
    }

    pub(super) fn parse_interpolated_string(
        &mut self,
        s: String,
        span: Span,
    ) -> Result<Expression> {
        let mut parts = Vec::with_capacity(4);
        let mut current = String::new();
        let mut chars = s.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '$' && chars.peek() == Some(&'{') {
                if !current.is_empty() {
                    parts.push(StringPart::Literal(current.clone()));
                    current.clear();
                }

                chars.next();

                let mut expr_str = String::new();
                let mut depth = 1;

                for ch in chars.by_ref() {
                    if ch == '{' {
                        depth += 1;
                        expr_str.push(ch);
                    } else if ch == '}' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                        expr_str.push(ch);
                    } else {
                        expr_str.push(ch);
                    }
                }

                use crate::lexer::tokenize;
                let tokens = tokenize(&expr_str)?;
                let mut expr_parser = Parser::new(tokens);
                let expr = expr_parser.parse_expression()?;

                parts.push(StringPart::Expression(expr));
            } else {
                current.push(ch);
            }
        }

        if !current.is_empty() {
            parts.push(StringPart::Literal(current));
        }

        Ok(Expression::InterpolatedString { parts, span })
    }
}
