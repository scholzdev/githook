use crate::error::{Span, LexError};

// ============================================================================
// SIMPLIFIED TOKEN SYSTEM
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Core Keywords
    Run,
    Block,
    Warn,
    Allow,
    Parallel,
    
    Let,
    Foreach,
    If,
    Else,
    Match,
    Where,
    Try,
    Catch,
    Break,
    Continue,
    
    Macro,
    Import,
    Use,
    Group,
    
    // Logical
    In,
    Not,
    And,
    Or,
    
    // Literals
    True,
    False,
    Null,
    
    // Comparison Operators
    Eq,        // ==
    Ne,        // !=
    Lt,        // <
    Le,        // <=
    Gt,        // >
    Ge,        // >=
    
    // Arithmetic Operators
    Plus,      // +
    Minus,     // -
    Star,      // *
    Slash,     // /
    Percent,   // %
    
    // Assignment
    Assign,    // =
    
    // Delimiters
    LeftBrace,     // {
    RightBrace,    // }
    LeftBracket,   // [
    RightBracket,  // ]
    LeftParen,     // (
    RightParen,    // )
    
    // Punctuation
    Dot,           // .
    Comma,         // ,
    Colon,         // :
    Arrow,         // ->
    FatArrow,      // =>
    
    // Special
    At,            // @ (for macros)
    Dollar,        // $ (for interpolation)
    
    // Values
    Identifier(String),
    String(String),
    Number(f64),
    
    // Metadata
    Newline,
    Comment(String),
}

#[derive(Debug, Clone)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Span,
}

// ============================================================================
// LEXER IMPLEMENTATION
// ============================================================================

pub fn tokenize(input: &str) -> Result<Vec<SpannedToken>, LexError> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    
    let mut line = 1;
    let mut col = 1;
    let mut offset = 0;
    
    let bump = |ch: char, line: &mut usize, col: &mut usize, offset: &mut usize| {
        if ch == '\n' {
            *line += 1;
            *col = 1;
        } else {
            *col += 1;
        }
        *offset += ch.len_utf8();
    };
    
    while let Some(&ch) = chars.peek() {
        let start_line = line;
        let start_col = col;
        let start_offset = offset;
        
        match ch {
            // Whitespace
            ' ' | '\t' | '\r' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
            }
            
            '\n' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                tokens.push(SpannedToken {
                    token: Token::Newline,
                    span: Span::new(start_line, start_col, start_offset, offset),
                });
            }
            
            // Comments
            '#' => {
                let mut comment = String::new();
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                
                while let Some(&ch) = chars.peek() {
                    if ch == '\n' {
                        break;
                    }
                    comment.push(ch);
                    chars.next();
                    bump(ch, &mut line, &mut col, &mut offset);
                }
                
                tokens.push(SpannedToken {
                    token: Token::Comment(comment),
                    span: Span::new(start_line, start_col, start_offset, offset),
                });
            }
            
            // Strings
            '"' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                
                let mut string = String::new();
                let mut escaped = false;
                
                while let Some(&ch) = chars.peek() {
                    if escaped {
                        string.push(match ch {
                            'n' => '\n',
                            't' => '\t',
                            'r' => '\r',
                            '\\' => '\\',
                            '"' => '"',
                            _ => ch,
                        });
                        escaped = false;
                    } else if ch == '\\' {
                        escaped = true;
                    } else if ch == '"' {
                        chars.next();
                        bump(ch, &mut line, &mut col, &mut offset);
                        break;
                    } else {
                        string.push(ch);
                    }
                    chars.next();
                    bump(ch, &mut line, &mut col, &mut offset);
                }
                
                tokens.push(SpannedToken {
                    token: Token::String(string),
                    span: Span::new(start_line, start_col, start_offset, offset),
                });
            }
            
            // Numbers
            '0'..='9' => {
                let mut num_str = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch.is_ascii_digit() || ch == '.' {
                        num_str.push(ch);
                        chars.next();
                        bump(ch, &mut line, &mut col, &mut offset);
                    } else {
                        break;
                    }
                }
                
                let mut num = num_str.parse::<f64>()
                    .map_err(|_| LexError::InvalidNumber {
                        text: num_str,
                        span: Span::new(start_line, start_col, start_offset, offset),
                    })?;
                
                // Check for size unit suffix (KB, MB, GB, TB)
                if let Some(&ch) = chars.peek() {
                    if ch.is_alphabetic() {
                        let mut unit = String::new();
                        while let Some(&ch) = chars.peek() {
                            if ch.is_alphabetic() {
                                unit.push(ch);
                                chars.next();
                                bump(ch, &mut line, &mut col, &mut offset);
                            } else {
                                break;
                            }
                        }
                        
                        // Apply multiplier based on unit
                        match unit.to_uppercase().as_str() {
                            "KB" => num *= 1024.0,
                            "MB" => num *= 1024.0 * 1024.0,
                            "GB" => num *= 1024.0 * 1024.0 * 1024.0,
                            "TB" => num *= 1024.0 * 1024.0 * 1024.0 * 1024.0,
                            _ => {
                                return Err(LexError::UnexpectedChar {
                                    ch: unit.chars().next().unwrap(),
                                    span: Span::new(start_line, start_col, start_offset, offset),
                                    suggestion: Some(format!("Unknown size unit: {}. Use KB, MB, GB, or TB", unit)),
                                });
                            }
                        }
                    }
                }
                
                tokens.push(SpannedToken {
                    token: Token::Number(num),
                    span: Span::new(start_line, start_col, start_offset, offset),
                });
            }
            
            // Operators
            '=' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                
                if chars.peek() == Some(&'=') {
                    chars.next();
                    bump('=', &mut line, &mut col, &mut offset);
                    tokens.push(SpannedToken {
                        token: Token::Eq,
                        span: Span::new(start_line, start_col, start_offset, offset),
                    });
                } else if chars.peek() == Some(&'>') {
                    chars.next();
                    bump('>', &mut line, &mut col, &mut offset);
                    tokens.push(SpannedToken {
                        token: Token::FatArrow,
                        span: Span::new(start_line, start_col, start_offset, offset),
                    });
                } else {
                    // Single = for assignment
                    tokens.push(SpannedToken {
                        token: Token::Assign,
                        span: Span::new(start_line, start_col, start_offset, offset),
                    });
                }
            }
            
            '!' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                
                if chars.peek() == Some(&'=') {
                    chars.next();
                    bump('=', &mut line, &mut col, &mut offset);
                    tokens.push(SpannedToken {
                        token: Token::Ne,
                        span: Span::new(start_line, start_col, start_offset, offset),
                    });
                } else {
                    return Err(LexError::UnexpectedChar {
                        ch: '!',
                        span: Span::new(start_line, start_col, start_offset, offset),
                        suggestion: Some("Did you mean '!='?".to_string()),
                    });
                }
            }
            
            '<' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                
                if chars.peek() == Some(&'=') {
                    chars.next();
                    bump('=', &mut line, &mut col, &mut offset);
                    tokens.push(SpannedToken {
                        token: Token::Le,
                        span: Span::new(start_line, start_col, start_offset, offset),
                    });
                } else {
                    tokens.push(SpannedToken {
                        token: Token::Lt,
                        span: Span::new(start_line, start_col, start_offset, offset),
                    });
                }
            }
            
            '>' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                
                if chars.peek() == Some(&'=') {
                    chars.next();
                    bump('=', &mut line, &mut col, &mut offset);
                    tokens.push(SpannedToken {
                        token: Token::Ge,
                        span: Span::new(start_line, start_col, start_offset, offset),
                    });
                } else {
                    tokens.push(SpannedToken {
                        token: Token::Gt,
                        span: Span::new(start_line, start_col, start_offset, offset),
                    });
                }
            }
            
            '-' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                
                if chars.peek() == Some(&'>') {
                    chars.next();
                    bump('>', &mut line, &mut col, &mut offset);
                    tokens.push(SpannedToken {
                        token: Token::Arrow,
                        span: Span::new(start_line, start_col, start_offset, offset),
                    });
                } else {
                    // Just minus operator
                    tokens.push(SpannedToken {
                        token: Token::Minus,
                        span: Span::new(start_line, start_col, start_offset, offset),
                    });
                }
            }
            
            '+' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                tokens.push(SpannedToken {
                    token: Token::Plus,
                    span: Span::new(start_line, start_col, start_offset, offset),
                });
            }
            
            '*' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                tokens.push(SpannedToken {
                    token: Token::Star,
                    span: Span::new(start_line, start_col, start_offset, offset),
                });
            }
            
            '/' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                
                // Check for comment
                if chars.peek() == Some(&'/') {
                    // Line comment
                    chars.next();
                    bump('/', &mut line, &mut col, &mut offset);
                    
                    let mut comment = String::new();
                    while let Some(&ch) = chars.peek() {
                        if ch == '\n' {
                            break;
                        }
                        chars.next();
                        bump(ch, &mut line, &mut col, &mut offset);
                        comment.push(ch);
                    }
                    
                    tokens.push(SpannedToken {
                        token: Token::Comment(comment.trim().to_string()),
                        span: Span::new(start_line, start_col, start_offset, offset),
                    });
                } else {
                    // Division operator
                    tokens.push(SpannedToken {
                        token: Token::Slash,
                        span: Span::new(start_line, start_col, start_offset, offset),
                    });
                }
            }
            
            '%' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                tokens.push(SpannedToken {
                    token: Token::Percent,
                    span: Span::new(start_line, start_col, start_offset, offset),
                });
            }
            
            // Single-char tokens
            '{' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                tokens.push(SpannedToken {
                    token: Token::LeftBrace,
                    span: Span::new(start_line, start_col, start_offset, offset),
                });
            }
            '}' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                tokens.push(SpannedToken {
                    token: Token::RightBrace,
                    span: Span::new(start_line, start_col, start_offset, offset),
                });
            }
            '[' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                tokens.push(SpannedToken {
                    token: Token::LeftBracket,
                    span: Span::new(start_line, start_col, start_offset, offset),
                });
            }
            ']' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                tokens.push(SpannedToken {
                    token: Token::RightBracket,
                    span: Span::new(start_line, start_col, start_offset, offset),
                });
            }
            '(' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                tokens.push(SpannedToken {
                    token: Token::LeftParen,
                    span: Span::new(start_line, start_col, start_offset, offset),
                });
            }
            ')' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                tokens.push(SpannedToken {
                    token: Token::RightParen,
                    span: Span::new(start_line, start_col, start_offset, offset),
                });
            }
            '.' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                tokens.push(SpannedToken {
                    token: Token::Dot,
                    span: Span::new(start_line, start_col, start_offset, offset),
                });
            }
            ',' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                tokens.push(SpannedToken {
                    token: Token::Comma,
                    span: Span::new(start_line, start_col, start_offset, offset),
                });
            }
            ':' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                tokens.push(SpannedToken {
                    token: Token::Colon,
                    span: Span::new(start_line, start_col, start_offset, offset),
                });
            }
            '@' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                tokens.push(SpannedToken {
                    token: Token::At,
                    span: Span::new(start_line, start_col, start_offset, offset),
                });
            }
            '$' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                tokens.push(SpannedToken {
                    token: Token::Dollar,
                    span: Span::new(start_line, start_col, start_offset, offset),
                });
            }
            
            // Identifiers and keywords
            _ if ch.is_alphabetic() || ch == '_' => {
                let mut ident = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch.is_alphanumeric() || ch == '_' {
                        ident.push(ch);
                        chars.next();
                        bump(ch, &mut line, &mut col, &mut offset);
                    } else {
                        break;
                    }
                }
                
                let token = match ident.as_str() {
                    "run" => Token::Run,
                    "block" => Token::Block,
                    "warn" => Token::Warn,
                    "allow" => Token::Allow,
                    "parallel" => Token::Parallel,
                    "let" => Token::Let,
                    "foreach" => Token::Foreach,
                    "if" => Token::If,
                    "else" => Token::Else,
                    "match" => Token::Match,
                    "where" => Token::Where,
                    "try" => Token::Try,
                    "catch" => Token::Catch,
                    "break" => Token::Break,
                    "continue" => Token::Continue,
                    "macro" => Token::Macro,
                    "import" => Token::Import,
                    "use" => Token::Use,
                    "group" => Token::Group,
                    "in" => Token::In,
                    "not" => Token::Not,
                    "and" => Token::And,
                    "or" => Token::Or,
                    "true" => Token::True,
                    "false" => Token::False,
                    "null" => Token::Null,
                    _ => Token::Identifier(ident),
                };
                
                tokens.push(SpannedToken {
                    token,
                    span: Span::new(start_line, start_col, start_offset, offset),
                });
            }
            
            _ => {
                return Err(LexError::UnexpectedChar {
                    ch,
                    span: Span::new(start_line, start_col, start_offset, offset),
                    suggestion: None,
                });
            }
        }
    }
    
    Ok(tokens)
}
