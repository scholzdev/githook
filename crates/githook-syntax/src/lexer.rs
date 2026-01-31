use crate::error::{Span, LexError};
use std::collections::HashMap;
use once_cell::sync::Lazy;

// Pre-computed keyword map for O(1) lookups instead of linear match
static KEYWORDS: Lazy<HashMap<&'static str, Token>> = Lazy::new(|| {
    let mut m = HashMap::with_capacity(32);
    m.insert("run", Token::Run);
    m.insert("print", Token::Print);
    m.insert("block", Token::Block);
    m.insert("warn", Token::Warn);
    m.insert("allow", Token::Allow);
    m.insert("parallel", Token::Parallel);
    m.insert("let", Token::Let);
    m.insert("foreach", Token::Foreach);
    m.insert("if", Token::If);
    m.insert("else", Token::Else);
    m.insert("match", Token::Match);
    m.insert("matching", Token::Matching);
    m.insert("try", Token::Try);
    m.insert("catch", Token::Catch);
    m.insert("break", Token::Break);
    m.insert("continue", Token::Continue);
    m.insert("macro", Token::Macro);
    m.insert("import", Token::Import);
    m.insert("use", Token::Use);
    m.insert("group", Token::Group);
    m.insert("in", Token::In);
    m.insert("not", Token::Not);
    m.insert("and", Token::And);
    m.insert("or", Token::Or);
    m.insert("true", Token::True);
    m.insert("false", Token::False);
    m.insert("null", Token::Null);
    m
});

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Run,
    Print,
    Block,
    Warn,
    Allow,
    Parallel,
    Let,
    Foreach,
    If,
    Else,
    Match,
    Matching,
    Try,
    Catch,
    Break,
    Continue,
    Macro,
    Import,
    Use,
    Group,
    In,
    Not,
    And,
    Or,
    True,
    False,
    Null,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Assign,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    LeftParen,
    RightParen,
    Dot,
    Comma,
    Colon,
    Arrow,
    FatArrow,
    At,
    Dollar, 
    Identifier(String),
    String(String),
    Number(f64),
    Newline,
    Comment(String),
}

#[derive(Debug, Clone)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Span,
}

pub fn tokenize(input: &str) -> Result<Vec<SpannedToken>, LexError> {
    let mut tokens = Vec::with_capacity(input.len() / 4); // Estimate ~4 chars per token
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
                
                if let Some(&ch) = chars.peek()
                    && ch.is_alphabetic() {
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
                
                tokens.push(SpannedToken {
                    token: Token::Number(num),
                    span: Span::new(start_line, start_col, start_offset, offset),
                });
            }
            
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
                
                if chars.peek() == Some(&'/') {
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
            
            _ if ch.is_alphabetic() || ch == '_' => {
                let mut ident = String::with_capacity(16); // Pre-allocate reasonable size
                while let Some(&ch) = chars.peek() {
                    if ch.is_alphanumeric() || ch == '_' {
                        ident.push(ch);
                        chars.next();
                        bump(ch, &mut line, &mut col, &mut offset);
                    } else {
                        break;
                    }
                }
                
                // Fast HashMap lookup instead of linear match
                let token = KEYWORDS.get(ident.as_str())
                    .cloned()
                    .unwrap_or_else(|| Token::Identifier(ident));
                
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
