//! Statement parsing methods for the recursive-descent parser.

use super::Parser;
use crate::ast::*;
use crate::lexer::{SpannedToken, Token};
use anyhow::{Result, bail};
use smallvec::SmallVec;

impl Parser {
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
                let token_str = other
                    .as_ref()
                    .map(|t| t.to_string())
                    .unwrap_or_else(|| "end of input".to_string());
                bail!("Unexpected token at statement level: {}", token_str)
            }
        }
    }

    fn parse_run(&mut self) -> Result<Statement> {
        let start_span = self.expect(Token::Run)?;

        let command = self.parse_expression()?;

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
                    Some(SpannedToken {
                        token: Token::String(s),
                        ..
                    }) => Some(s),
                    other => bail!("Expected string after 'message', got {:?}", other),
                };
            }

            if matches!(self.peek(), Some(Token::Identifier(id)) if id == "interactive") {
                self.advance();
                interactive = match self.advance() {
                    Some(SpannedToken {
                        token: Token::String(s),
                        ..
                    }) => Some(s),
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
                Some(SpannedToken {
                    token: Token::String(s),
                    ..
                }) => s,
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
                    Some(SpannedToken {
                        token: Token::String(s),
                        ..
                    }) => Some(s),
                    other => bail!("Expected string after 'message', got {:?}", other),
                };
            }

            if matches!(self.peek(), Some(Token::Identifier(id)) if id == "interactive") {
                self.advance();
                interactive = match self.advance() {
                    Some(SpannedToken {
                        token: Token::String(s),
                        ..
                    }) => Some(s),
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
                Some(SpannedToken {
                    token: Token::String(s),
                    ..
                }) => s,
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
            Some(SpannedToken {
                token: Token::String(s),
                ..
            }) => s,
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

        let mut commands = Vec::with_capacity(8);

        self.skip_newlines();

        while !matches!(self.peek(), Some(Token::RightBrace)) {
            self.expect(Token::Run)?;

            let cmd = match self.advance() {
                Some(SpannedToken {
                    token: Token::String(s),
                    ..
                }) => s,
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
            Some(SpannedToken {
                token: Token::Identifier(id),
                ..
            }) => id,
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
                Some(SpannedToken {
                    token: Token::String(s),
                    ..
                }) => Some(s),
                other => bail!("Expected string pattern after 'matching', got {:?}", other),
            }
        } else {
            None
        };

        self.expect(Token::LeftBrace)?;
        self.skip_newlines();

        let var = match self.advance() {
            Some(SpannedToken {
                token: Token::Identifier(id),
                ..
            }) => id,
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

        let mut arms = Vec::with_capacity(4);

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

        self.expect(Token::LeftBrace)?;
        self.skip_newlines();

        // Optional: `catch { err in ... }` — binds the error to a variable.
        let catch_var = if matches!(self.peek(), Some(Token::Identifier(_))) {
            // Peek ahead to check for `in` after the identifier.
            if matches!(self.peek_nth(1), Some(Token::In)) {
                let var = match self.advance() {
                    Some(SpannedToken {
                        token: Token::Identifier(id),
                        ..
                    }) => id,
                    _ => unreachable!(),
                };
                self.expect(Token::In)?;
                self.skip_newlines();
                Some(var)
            } else {
                None
            }
        } else {
            None
        };

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
        let pattern_start = self
            .peek_span()
            .ok_or_else(|| anyhow::anyhow!("unexpected end of input in match arm"))?;

        let pattern = match self.peek() {
            Some(Token::Identifier(id)) if id == "_" => {
                self.advance();
                MatchPattern::Underscore(pattern_start)
            }

            Some(Token::String(_)) => {
                if let Some(SpannedToken {
                    token: Token::String(s),
                    span,
                }) = self.advance()
                {
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
            Some(SpannedToken {
                token: Token::Identifier(id),
                ..
            }) => id,
            other => bail!("Expected identifier after 'macro', got {:?}", other),
        };

        let params = if matches!(self.peek(), Some(Token::LeftParen)) {
            self.advance();
            let mut params = Vec::with_capacity(4);

            while !matches!(self.peek(), Some(Token::RightParen)) {
                let param = match self.advance() {
                    Some(SpannedToken {
                        token: Token::Identifier(id),
                        ..
                    }) => id,
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
            Some(SpannedToken {
                token: Token::Identifier(id),
                ..
            }) => id,
            other => bail!("Expected identifier after '@', got {:?}", other),
        };

        let final_name = if matches!(self.peek(), Some(Token::Dot)) {
            self.advance();
            namespace = Some(name);
            match self.advance() {
                Some(SpannedToken {
                    token: Token::Identifier(id),
                    ..
                }) => id,
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
            Some(SpannedToken {
                token: Token::String(s),
                ..
            }) => s,
            other => bail!("Expected string after 'import', got {:?}", other),
        };

        let alias = if matches!(self.peek(), Some(Token::Identifier(id)) if id == "as") {
            self.advance();
            Some(match self.advance() {
                Some(SpannedToken {
                    token: Token::Identifier(id),
                    ..
                }) => id,
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
            Some(SpannedToken {
                token: Token::String(s),
                ..
            }) => s,
            other => bail!("Expected string after 'use', got {:?}", other),
        };

        let alias = if matches!(self.peek(), Some(Token::Identifier(id)) if id == "as") {
            self.advance();
            Some(match self.advance() {
                Some(SpannedToken {
                    token: Token::Identifier(id),
                    ..
                }) => id,
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
            Some(SpannedToken {
                token: Token::Identifier(id),
                ..
            }) => id,
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
            _ => Some(Severity::Critical),
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
}
