use crate::ast::*;
use crate::lexer::{Token, SpannedToken};
use crate::error::Span;
use anyhow::{Result, bail};
use smallvec::SmallVec;

pub struct Parser {
    tokens: Vec<SpannedToken>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<SpannedToken>) -> Self {
        Self { tokens, pos: 0 }
    }

    #[inline]
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|st| &st.token)
    }

    #[inline]
    fn peek_span(&self) -> Option<Span> {
        self.tokens.get(self.pos).map(|st| st.span)
    }

    #[inline]
    fn advance(&mut self) -> Option<SpannedToken> {
        if self.pos < self.tokens.len() {
            let token = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(token)
        } else {
            None
        }
    }

    fn expect(&mut self, expected: Token) -> Result<Span> {
        match self.advance() {
            Some(st) if st.token == expected => Ok(st.span),
            Some(st) => bail!("Expected {:?}, got {:?} at {:?}", expected, st.token, st.span),
            None => bail!("Expected {:?}, got EOF", expected),
        }
    }

    #[inline]
    fn skip_newlines(&mut self) {
        while matches!(self.peek(), Some(Token::Newline) | Some(Token::Comment(_))) {
            self.advance();
        }
    }

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
            Some(Token::Eq) => { self.advance(); BinaryOp::Eq }
            Some(Token::Ne) => { self.advance(); BinaryOp::Ne }
            Some(Token::Lt) => { self.advance(); BinaryOp::Lt }
            Some(Token::Le) => { self.advance(); BinaryOp::Le }
            Some(Token::Gt) => { self.advance(); BinaryOp::Gt }
            Some(Token::Ge) => { self.advance(); BinaryOp::Ge }
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
                Some(Token::Plus) => { self.advance(); BinaryOp::Add }
                Some(Token::Minus) => { self.advance(); BinaryOp::Sub }
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
        
        while matches!(self.peek(), Some(Token::Star) | Some(Token::Slash) | Some(Token::Percent)) {
            let op = match self.peek() {
                Some(Token::Star) => { self.advance(); BinaryOp::Mul }
                Some(Token::Slash) => { self.advance(); BinaryOp::Div }
                Some(Token::Percent) => { self.advance(); BinaryOp::Mod }
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
            let start_span = self.advance().unwrap().span;
            let expr = self.parse_unary()?;
            let span = start_span.merge(expr.span());
            
            return Ok(Expression::Unary {
                op: UnaryOp::Not,
                expr: Box::new(expr),
                span,
            });
        }
        
        if matches!(self.peek(), Some(Token::Minus)) {
            let start_span = self.advance().unwrap().span;
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
                        Some(SpannedToken { token: Token::Identifier(id), .. }) => id,
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
                        let span = *expr.span();
                        let mut chain = match expr {
                            Expression::Identifier(id, _) => SmallVec::from_vec(vec![id]),
                            Expression::PropertyAccess { chain, .. } => chain,
                            _ => bail!("Cannot access property on non-identifier expression"),
                        };
                        
                        chain.push(name);
                        
                        expr = Expression::PropertyAccess { chain, span };
                    }
                }
                Token::LeftParen => {
                    // Handle function calls like: file("path")
                    // Convert to MethodCall with identifier as pseudo-receiver
                    if let Expression::Identifier(name, id_span) = expr {
                        self.advance(); // consume '('
                        
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
                        
                        // Represent as MethodCall with Identifier as receiver
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
                _ => break,
            }
        }
        
        Ok(expr)
    }
    
    fn parse_primary(&mut self) -> Result<Expression> {
        match self.peek() {
            Some(Token::True) => {
                let span = self.advance().unwrap().span;
                Ok(Expression::Bool(true, span))
            }
            
            Some(Token::False) => {
                let span = self.advance().unwrap().span;
                Ok(Expression::Bool(false, span))
            }
            
            Some(Token::Null) => {
                let span = self.advance().unwrap().span;
                Ok(Expression::Null(span))
            }
            
            Some(Token::Number(_)) => {
                let st = self.advance().unwrap();
                if let Token::Number(n) = st.token {
                    Ok(Expression::Number(n, st.span))
                } else {
                    unreachable!()
                }
            }
            
            Some(Token::String(_)) => {
                let st = self.advance().unwrap();
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
                let st = self.advance().unwrap();
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
                let start_span = self.advance().unwrap().span;
                let mut items = Vec::with_capacity(8); // Pre-allocate for typical arrays
                
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
            
            other => {
                let token_str = other.as_ref().map(|t| t.to_string()).unwrap_or_else(|| "end of input".to_string());
                bail!("Unexpected token in expression: {}", token_str)
            }
        }
    }
    
    fn parse_interpolated_string(&mut self, s: String, span: Span) -> Result<Expression> {
        let mut parts = Vec::with_capacity(4); // Pre-allocate for typical interpolations
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

    pub fn parse_statement(&mut self) -> Result<Statement> {
        self.skip_newlines();
        
        match self.peek() {
            Some(Token::Run) => self.parse_run(),
            Some(Token::Print) => self.parse_print(),
            Some(Token::Block) => self.parse_block_or_blockif(),
            Some(Token::Warn) => self.parse_warn_or_warnif(),
            Some(Token::Allow) => self.parse_allow(),
            Some(Token::Parallel) => self.parse_parallel(),
            Some(Token::Let) => self.parse_let(),
            Some(Token::Foreach) => self.parse_foreach(),
            Some(Token::If) => self.parse_if(),
            Some(Token::Match) => self.parse_match(),
            Some(Token::Try) => self.parse_try(),
            Some(Token::Break) => self.parse_break(),
            Some(Token::Continue) => self.parse_continue(),
            Some(Token::Macro) => self.parse_macro_def(),
            Some(Token::At) => self.parse_macro_call(),
            Some(Token::Import) => self.parse_import(),
            Some(Token::Use) => self.parse_use(),
            Some(Token::Group) => self.parse_group(),
            other => {
                let token_str = other.as_ref().map(|t| t.to_string()).unwrap_or_else(|| "end of input".to_string());
                bail!("Unexpected token at statement level: {}", token_str)
            }
        }
    }
    
    fn parse_run(&mut self) -> Result<Statement> {
        let start_span = self.expect(Token::Run)?;
        
        let command = match self.advance() {
            Some(SpannedToken { token: Token::String(s), .. }) => s,
            other => bail!("Expected string after 'run', got {:?}", other),
        };
        
        Ok(Statement::Run {
            command,
            span: start_span,
        })
    }
    
    fn parse_print(&mut self) -> Result<Statement> {
        let start_span = self.expect(Token::Print)?;
        
        let message = self.parse_expression()?;
        
        Ok(Statement::Print {
            message,
            span: start_span,
        })
    }
    
    fn parse_block_or_blockif(&mut self) -> Result<Statement> {
        let start_span = self.expect(Token::Block)?;
        
        if matches!(self.peek(), Some(Token::If)) {
            self.advance();
            
            let condition = self.parse_expression()?;
            
            let mut message = None;
            let mut interactive = None;
            
            if matches!(self.peek(), Some(Token::Identifier(id)) if id == "message") {
                self.advance();
                message = match self.advance() {
                    Some(SpannedToken { token: Token::String(s), .. }) => Some(s),
                    other => bail!("Expected string after 'message', got {:?}", other),
                };
            }
            
            if matches!(self.peek(), Some(Token::Identifier(id)) if id == "interactive") {
                self.advance();
                interactive = match self.advance() {
                    Some(SpannedToken { token: Token::String(s), .. }) => Some(s),
                    other => bail!("Expected string after 'interactive', got {:?}", other),
                };
            }
            
            Ok(Statement::BlockIf {
                condition,
                message,
                interactive,
                span: start_span,
            })
        } else {
            let message = match self.advance() {
                Some(SpannedToken { token: Token::String(s), .. }) => s,
                other => bail!("Expected string after 'block', got {:?}", other),
            };
            
            Ok(Statement::Block {
                message,
                span: start_span,
            })
        }
    }
    
    fn parse_warn_or_warnif(&mut self) -> Result<Statement> {
        let start_span = self.expect(Token::Warn)?;
        
        if matches!(self.peek(), Some(Token::If)) {
            self.advance();
            
            let condition = self.parse_expression()?;
            
            let mut message = None;
            let mut interactive = None;
            
            if matches!(self.peek(), Some(Token::Identifier(id)) if id == "message") {
                self.advance();
                message = match self.advance() {
                    Some(SpannedToken { token: Token::String(s), .. }) => Some(s),
                    other => bail!("Expected string after 'message', got {:?}", other),
                };
            }
            
            if matches!(self.peek(), Some(Token::Identifier(id)) if id == "interactive") {
                self.advance();
                interactive = match self.advance() {
                    Some(SpannedToken { token: Token::String(s), .. }) => Some(s),
                    other => bail!("Expected string after 'interactive', got {:?}", other),
                };
            }
            
            Ok(Statement::WarnIf {
                condition,
                message,
                interactive,
                span: start_span,
            })
        } else {
            let message = match self.advance() {
                Some(SpannedToken { token: Token::String(s), .. }) => s,
                other => bail!("Expected string after 'warn', got {:?}", other),
            };
            
            Ok(Statement::Warn {
                message,
                span: start_span,
            })
        }
    }
    
    fn parse_allow(&mut self) -> Result<Statement> {
        let start_span = self.expect(Token::Allow)?;
        
        let command = match self.advance() {
            Some(SpannedToken { token: Token::String(s), .. }) => s,
            other => bail!("Expected string after 'allow', got {:?}", other),
        };
        
        Ok(Statement::Allow {
            command,
            span: start_span,
        })
    }
    
    fn parse_parallel(&mut self) -> Result<Statement> {
        let start_span = self.expect(Token::Parallel)?;
        self.expect(Token::LeftBrace)?;
        
        let mut commands = Vec::with_capacity(8); // Pre-allocate for typical parallel blocks
        
        self.skip_newlines();
        
        while !matches!(self.peek(), Some(Token::RightBrace)) {
            self.expect(Token::Run)?;
            
            let cmd = match self.advance() {
                Some(SpannedToken { token: Token::String(s), .. }) => s,
                other => bail!("Expected string after 'run', got {:?}", other),
            };
            
            commands.push(cmd);
            self.skip_newlines();
        }
        
        self.expect(Token::RightBrace)?;
        
        Ok(Statement::Parallel {
            commands: SmallVec::from_vec(commands),
            span: start_span,
        })
    }
    
    fn parse_let(&mut self) -> Result<Statement> {
        let start_span = self.expect(Token::Let)?;
        
        let name = match self.advance() {
            Some(SpannedToken { token: Token::Identifier(id), .. }) => id,
            other => bail!("Expected identifier after 'let', got {:?}", other),
        };
        
        self.expect(Token::Assign)?;
        
        let expr = self.parse_expression()?;
        let value = LetValue::Expression(expr);
        
        Ok(Statement::Let {
            name,
            value,
            span: start_span,
        })
    }
    
    fn parse_break(&mut self) -> Result<Statement> {
        let span = self.expect(Token::Break)?;
        Ok(Statement::Break { span })
    }
    
    fn parse_continue(&mut self) -> Result<Statement> {
        let span = self.expect(Token::Continue)?;
        Ok(Statement::Continue { span })
    }
    
    fn parse_foreach(&mut self) -> Result<Statement> {
        let start_span = self.expect(Token::Foreach)?;
        
        let collection = self.parse_expression()?;
        
        let pattern = if matches!(self.peek(), Some(Token::Matching)) {
            self.advance();
            match self.advance() {
                Some(SpannedToken { token: Token::String(s), .. }) => Some(s),
                other => bail!("Expected string pattern after 'matching', got {:?}", other),
            }
        } else {
            None
        };
        
        self.expect(Token::LeftBrace)?;
        self.skip_newlines();
        
        let var = match self.advance() {
            Some(SpannedToken { token: Token::Identifier(id), .. }) => id,
            other => bail!("Expected identifier for loop variable, got {:?}", other),
        };
        
        self.expect(Token::In)?;
        self.skip_newlines();
        
        let body = self.parse_body()?;
        
        self.expect(Token::RightBrace)?;
        
        Ok(Statement::ForEach {
            collection,
            var,
            pattern,
            body,
            span: start_span,
        })
    }
    
    fn parse_if(&mut self) -> Result<Statement> {
        let start_span = self.expect(Token::If)?;
        
        let condition = self.parse_expression()?;
        
        self.expect(Token::LeftBrace)?;
        let then_body = self.parse_body()?;
        self.expect(Token::RightBrace)?;
        
        let else_body = if matches!(self.peek(), Some(Token::Else)) {
            self.advance();
            self.expect(Token::LeftBrace)?;
            let body = self.parse_body()?;
            self.expect(Token::RightBrace)?;
            Some(body)
        } else {
            None
        };
        
        Ok(Statement::If {
            condition,
            then_body,
            else_body,
            span: start_span,
        })
    }
    
    fn parse_match(&mut self) -> Result<Statement> {
        let start_span = self.expect(Token::Match)?;
        
        let subject = self.parse_expression()?;
        
        self.expect(Token::LeftBrace)?;
        self.skip_newlines();
        
        let mut arms = Vec::with_capacity(4); // Pre-allocate for typical match arms
        
        while !matches!(self.peek(), Some(Token::RightBrace)) {
            let arm = self.parse_match_arm()?;
            arms.push(arm);
            self.skip_newlines();
        }
        
        self.expect(Token::RightBrace)?;
        
        Ok(Statement::Match {
            subject,
            arms,
            span: start_span,
        })
    }
    
    fn parse_try(&mut self) -> Result<Statement> {
        let start_span = self.expect(Token::Try)?;
        
        self.expect(Token::LeftBrace)?;
        let body = self.parse_body()?;
        self.expect(Token::RightBrace)?;
        
        self.skip_newlines();
        self.expect(Token::Catch)?;
        
        let catch_var = if matches!(self.peek(), Some(Token::Identifier(_))) {
            if let Some(SpannedToken { token: Token::Identifier(var), .. }) = self.advance() {
                Some(var)
            } else {
                None
            }
        } else {
            None
        };
        
        self.expect(Token::LeftBrace)?;
        let catch_body = self.parse_body()?;
        self.expect(Token::RightBrace)?;
        
        Ok(Statement::Try {
            body,
            catch_var,
            catch_body,
            span: start_span,
        })
    }
    
    fn parse_match_arm(&mut self) -> Result<MatchArm> {
        let pattern_start = self.peek_span().unwrap();
        
        let pattern = match self.peek() {
            Some(Token::Identifier(id)) if id == "_" => {
                self.advance();
                MatchPattern::Underscore(pattern_start)
            }
            
            Some(Token::String(_)) => {
                if let Some(SpannedToken { token: Token::String(s), span }) = self.advance() {
                    MatchPattern::Wildcard(s, span)
                } else {
                    unreachable!()
                }
            }
            
            _ => {
                let expr = self.parse_expression()?;
                let span = *expr.span();
                MatchPattern::Expression(expr, span)
            }
        };
        
        self.expect(Token::Arrow)?;
        
        let body = if matches!(self.peek(), Some(Token::LeftBrace)) {
            self.advance();
            let stmts = self.parse_body()?;
            self.expect(Token::RightBrace)?;
            stmts
        } else {
            vec![self.parse_statement()?]
        };
        
        Ok(MatchArm {
            pattern,
            body,
            span: pattern_start,
        })
    }
    
    fn parse_macro_def(&mut self) -> Result<Statement> {
        let start_span = self.expect(Token::Macro)?;
        
        let name = match self.advance() {
            Some(SpannedToken { token: Token::Identifier(id), .. }) => id,
            other => bail!("Expected identifier after 'macro', got {:?}", other),
        };
        
        let params = if matches!(self.peek(), Some(Token::LeftParen)) {
            self.advance();
            let mut params = Vec::with_capacity(4); // Pre-allocate for typical macro params
            
            while !matches!(self.peek(), Some(Token::RightParen)) {
                let param = match self.advance() {
                    Some(SpannedToken { token: Token::Identifier(id), .. }) => id,
                    other => bail!("Expected parameter name, got {:?}", other),
                };
                
                params.push(param);
                
                if matches!(self.peek(), Some(Token::Comma)) {
                    self.advance();
                }
            }
            
            self.expect(Token::RightParen)?;
            params
        } else {
            Vec::new()
        };
        
        self.expect(Token::LeftBrace)?;
        let body = self.parse_body()?;
        self.expect(Token::RightBrace)?;
        
        Ok(Statement::MacroDef {
            name,
            params: SmallVec::from_vec(params),
            body,
            span: start_span,
        })
    }
    
    fn parse_macro_call(&mut self) -> Result<Statement> {
        let start_span = self.expect(Token::At)?;
        
        let mut namespace = None;
        let name = match self.advance() {
            Some(SpannedToken { token: Token::Identifier(id), .. }) => id,
            other => bail!("Expected identifier after '@', got {:?}", other),
        };
        
        let final_name = if matches!(self.peek(), Some(Token::Dot)) {
            self.advance();
            namespace = Some(name);
            match self.advance() {
                Some(SpannedToken { token: Token::Identifier(id), .. }) => id,
                other => bail!("Expected identifier after '.', got {:?}", other),
            }
        } else {
            name
        };
        
        let args = if matches!(self.peek(), Some(Token::LeftParen)) {
            self.advance();
            let mut args = Vec::new();
            
            while !matches!(self.peek(), Some(Token::RightParen)) {
                args.push(self.parse_expression()?);
                
                if matches!(self.peek(), Some(Token::Comma)) {
                    self.advance();
                }
            }
            
            self.expect(Token::RightParen)?;
            args
        } else {
            Vec::new()
        };
        
        Ok(Statement::MacroCall {
            namespace,
            name: final_name,
            args,
            span: start_span,
        })
    }
    
    fn parse_import(&mut self) -> Result<Statement> {
        let start_span = self.expect(Token::Import)?;
        
        let path = match self.advance() {
            Some(SpannedToken { token: Token::String(s), .. }) => s,
            other => bail!("Expected string after 'import', got {:?}", other),
        };
        
        let alias = if matches!(self.peek(), Some(Token::Identifier(id)) if id == "as") {
            self.advance();
            Some(match self.advance() {
                Some(SpannedToken { token: Token::Identifier(id), .. }) => id,
                other => bail!("Expected identifier after 'as', got {:?}", other),
            })
        } else {
            None
        };
        
        Ok(Statement::Import {
            path,
            alias,
            span: start_span,
        })
    }
    
    fn parse_use(&mut self) -> Result<Statement> {
        let start_span = self.expect(Token::Use)?;
        
        let package = match self.advance() {
            Some(SpannedToken { token: Token::String(s), .. }) => s,
            other => bail!("Expected string after 'use', got {:?}", other),
        };
        
        let alias = if matches!(self.peek(), Some(Token::Identifier(id)) if id == "as") {
            self.advance();
            Some(match self.advance() {
                Some(SpannedToken { token: Token::Identifier(id), .. }) => id,
                other => bail!("Expected identifier after 'as', got {:?}", other),
            })
        } else {
            None
        };
        
        Ok(Statement::Use {
            package,
            alias,
            span: start_span,
        })
    }
    
    fn parse_group(&mut self) -> Result<Statement> {
        let start_span = self.expect(Token::Group)?;
        
        let name = match self.advance() {
            Some(SpannedToken { token: Token::Identifier(id), .. }) => id,
            other => bail!("Expected identifier after 'group', got {:?}", other),
        };
        
        let severity = match self.peek() {
            Some(Token::Identifier(id)) if id == "critical" => {
                self.advance();
                Some(Severity::Critical)
            }
            Some(Token::Identifier(id)) if id == "warning" => {
                self.advance();
                Some(Severity::Warning)
            }
            Some(Token::Identifier(id)) if id == "info" => {
                self.advance();
                Some(Severity::Info)
            }
            _ => Some(Severity::Critical), // Default to critical
        };
        
        let enabled = if matches!(self.peek(), Some(Token::Identifier(id)) if id == "disabled") {
            self.advance();
            false
        } else if matches!(self.peek(), Some(Token::Identifier(id)) if id == "enabled") {
            self.advance();
            true
        } else {
            true
        };
        
        self.expect(Token::LeftBrace)?;
        let body = self.parse_body()?;
        self.expect(Token::RightBrace)?;
        
        Ok(Statement::Group {
            name,
            severity,
            enabled,
            body,
            span: start_span,
        })
    }
    
    fn parse_body(&mut self) -> Result<Vec<Statement>> {
        let mut statements = Vec::with_capacity(16); // Pre-allocate for typical blocks
        
        self.skip_newlines();
        
        while !matches!(self.peek(), Some(Token::RightBrace) | None) {
            statements.push(self.parse_statement()?);
            self.skip_newlines();
        }
        
        Ok(statements)
    }
}

pub fn parse(tokens: Vec<SpannedToken>) -> Result<Vec<Statement>> {
    let mut parser = Parser::new(tokens);
    let mut statements = Vec::with_capacity(32); // Pre-allocate for typical programs
    
    parser.skip_newlines();
    
    while parser.peek().is_some() {
        statements.push(parser.parse_statement()?);
        parser.skip_newlines();
    }
    
    Ok(statements)
}
