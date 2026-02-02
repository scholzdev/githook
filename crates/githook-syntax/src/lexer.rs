use crate::error::{LexError, Span};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::fmt;

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

impl Token {
    pub fn display_name(&self) -> String {
        match self {
            Token::Run => "keyword 'run'".to_string(),
            Token::Print => "keyword 'print'".to_string(),
            Token::Block => "keyword 'block'".to_string(),
            Token::Warn => "keyword 'warn'".to_string(),
            Token::Allow => "keyword 'allow'".to_string(),
            Token::Parallel => "keyword 'parallel'".to_string(),
            Token::Let => "keyword 'let'".to_string(),
            Token::Foreach => "keyword 'foreach'".to_string(),
            Token::If => "keyword 'if'".to_string(),
            Token::Else => "keyword 'else'".to_string(),
            Token::Match => "keyword 'match'".to_string(),
            Token::Matching => "keyword 'matching'".to_string(),
            Token::Try => "keyword 'try'".to_string(),
            Token::Catch => "keyword 'catch'".to_string(),
            Token::Break => "keyword 'break'".to_string(),
            Token::Continue => "keyword 'continue'".to_string(),
            Token::Macro => "keyword 'macro'".to_string(),
            Token::Import => "keyword 'import'".to_string(),
            Token::Use => "keyword 'use'".to_string(),
            Token::Group => "keyword 'group'".to_string(),
            Token::In => "keyword 'in'".to_string(),
            Token::Not => "keyword 'not'".to_string(),
            Token::And => "keyword 'and'".to_string(),
            Token::Or => "keyword 'or'".to_string(),
            Token::True => "keyword 'true'".to_string(),
            Token::False => "keyword 'false'".to_string(),
            Token::Null => "keyword 'null'".to_string(),
            Token::Eq => "'=='".to_string(),
            Token::Ne => "'!='".to_string(),
            Token::Lt => "'<'".to_string(),
            Token::Le => "'<='".to_string(),
            Token::Gt => "'>'".to_string(),
            Token::Ge => "'>='".to_string(),
            Token::Plus => "'+'".to_string(),
            Token::Minus => "'-'".to_string(),
            Token::Star => "'*'".to_string(),
            Token::Slash => "'/'".to_string(),
            Token::Percent => "'%'".to_string(),
            Token::Assign => "'='".to_string(),
            Token::LeftBrace => "'{'".to_string(),
            Token::RightBrace => "'}'".to_string(),
            Token::LeftBracket => "'['".to_string(),
            Token::RightBracket => "']'".to_string(),
            Token::LeftParen => "'('".to_string(),
            Token::RightParen => "')'".to_string(),
            Token::Dot => "'.'".to_string(),
            Token::Comma => "','".to_string(),
            Token::Colon => "':'".to_string(),
            Token::Arrow => "'->'".to_string(),
            Token::FatArrow => "'=>'".to_string(),
            Token::At => "'@'".to_string(),
            Token::Dollar => "'$'".to_string(),
            Token::Identifier(s) => format!("'{}'", s),
            Token::String(s) => format!("string \"{}\"", s),
            Token::Number(n) => format!("number {}", n),
            Token::Newline => "newline".to_string(),
            Token::Comment(_) => "comment".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Span,
}

pub fn tokenize(input: &str) -> Result<Vec<SpannedToken>, LexError> {
    let mut tokens = Vec::with_capacity(input.len() / 4);
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

                let mut num = num_str
                    .parse::<f64>()
                    .map_err(|_| LexError::InvalidNumber {
                        text: num_str,
                        span: Span::new(start_line, start_col, start_offset, offset),
                    })?;

                if let Some(&ch) = chars.peek()
                    && ch.is_alphabetic()
                {
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
                                suggestion: Some(format!(
                                    "Unknown size unit: {}. Use KB, MB, GB, or TB",
                                    unit
                                )),
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
                let mut ident = String::with_capacity(16);
                while let Some(&ch) = chars.peek() {
                    if ch.is_alphanumeric() || ch == '_' {
                        ident.push(ch);
                        chars.next();
                        bump(ch, &mut line, &mut col, &mut offset);
                    } else {
                        break;
                    }
                }

                let token = KEYWORDS
                    .get(ident.as_str())
                    .cloned()
                    .unwrap_or(Token::Identifier(ident));

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

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Token::Run => write!(f, "run"),
            Token::Print => write!(f, "print"),
            Token::Block => write!(f, "block"),
            Token::Warn => write!(f, "warn"),
            Token::Allow => write!(f, "allow"),
            Token::Parallel => write!(f, "parallel"),
            Token::Let => write!(f, "let"),
            Token::Foreach => write!(f, "foreach"),
            Token::If => write!(f, "if"),
            Token::Else => write!(f, "else"),
            Token::Match => write!(f, "match"),
            Token::Matching => write!(f, "matching"),
            Token::Try => write!(f, "try"),
            Token::Catch => write!(f, "catch"),
            Token::Break => write!(f, "break"),
            Token::Continue => write!(f, "continue"),
            Token::Macro => write!(f, "macro"),
            Token::Import => write!(f, "import"),
            Token::Use => write!(f, "use"),
            Token::Group => write!(f, "group"),
            Token::In => write!(f, "in"),
            Token::Not => write!(f, "not"),
            Token::And => write!(f, "and"),
            Token::Or => write!(f, "or"),
            Token::True => write!(f, "true"),
            Token::False => write!(f, "false"),
            Token::Null => write!(f, "null"),
            Token::Eq => write!(f, "=="),
            Token::Ne => write!(f, "!="),
            Token::Lt => write!(f, "<"),
            Token::Le => write!(f, "<="),
            Token::Gt => write!(f, ">"),
            Token::Ge => write!(f, ">="),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Star => write!(f, "*"),
            Token::Slash => write!(f, "/"),
            Token::Percent => write!(f, "%"),
            Token::Assign => write!(f, "="),
            Token::LeftBrace => write!(f, "{{"),
            Token::RightBrace => write!(f, "}}"),
            Token::LeftBracket => write!(f, "["),
            Token::RightBracket => write!(f, "]"),
            Token::LeftParen => write!(f, "("),
            Token::RightParen => write!(f, ")"),
            Token::Dot => write!(f, "."),
            Token::Comma => write!(f, ","),
            Token::Colon => write!(f, ":"),
            Token::Arrow => write!(f, "->"),
            Token::FatArrow => write!(f, "=>"),
            Token::At => write!(f, "@"),
            Token::Dollar => write!(f, "$"),
            Token::Identifier(s) => write!(f, "{}", s),
            Token::String(s) => write!(f, "\"{}\"", s),
            Token::Number(n) => write!(f, "{}", n),
            Token::Newline => write!(f, "newline"),
            Token::Comment(s) => write!(f, "# {}", s),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_keywords() {
        let input = "run print block warn if else group macro";
        let tokens = tokenize(input).unwrap();
        
        assert_eq!(tokens[0].token, Token::Run);
        assert_eq!(tokens[1].token, Token::Print);
        assert_eq!(tokens[2].token, Token::Block);
        assert_eq!(tokens[3].token, Token::Warn);
        assert_eq!(tokens[4].token, Token::If);
        assert_eq!(tokens[5].token, Token::Else);
        assert_eq!(tokens[6].token, Token::Group);
        assert_eq!(tokens[7].token, Token::Macro);
    }

    #[test]
    fn test_tokenize_operators() {
        let input = "== != < <= > >= + - * / %";
        let tokens = tokenize(input).unwrap();
        
        assert_eq!(tokens[0].token, Token::Eq);
        assert_eq!(tokens[1].token, Token::Ne);
        assert_eq!(tokens[2].token, Token::Lt);
        assert_eq!(tokens[3].token, Token::Le);
        assert_eq!(tokens[4].token, Token::Gt);
        assert_eq!(tokens[5].token, Token::Ge);
        assert_eq!(tokens[6].token, Token::Plus);
        assert_eq!(tokens[7].token, Token::Minus);
        assert_eq!(tokens[8].token, Token::Star);
        assert_eq!(tokens[9].token, Token::Slash);
        assert_eq!(tokens[10].token, Token::Percent);
    }

    #[test]
    fn test_tokenize_brackets() {
        let input = "{ } [ ] ( )";
        let tokens = tokenize(input).unwrap();
        
        assert_eq!(tokens[0].token, Token::LeftBrace);
        assert_eq!(tokens[1].token, Token::RightBrace);
        assert_eq!(tokens[2].token, Token::LeftBracket);
        assert_eq!(tokens[3].token, Token::RightBracket);
        assert_eq!(tokens[4].token, Token::LeftParen);
        assert_eq!(tokens[5].token, Token::RightParen);
    }

    #[test]
    fn test_tokenize_string() {
        let input = r#""hello world""#;
        let tokens = tokenize(input).unwrap();
        
        assert_eq!(tokens[0].token, Token::String("hello world".to_string()));
    }

    #[test]
    fn test_tokenize_string_escaped() {
        let input = r#""hello \"quoted\" world""#;
        let tokens = tokenize(input).unwrap();
        
        assert_eq!(tokens[0].token, Token::String("hello \"quoted\" world".to_string()));
    }

    #[test]
    fn test_tokenize_number() {
        let input = "42 3.14 0.5";
        let tokens = tokenize(input).unwrap();
        
        assert_eq!(tokens[0].token, Token::Number(42.0));
        assert_eq!(tokens[1].token, Token::Number(3.14));
        assert_eq!(tokens[2].token, Token::Number(0.5));
    }

    #[test]
    fn test_tokenize_identifier() {
        let input = "myVar userName file_path";
        let tokens = tokenize(input).unwrap();
        
        assert_eq!(tokens[0].token, Token::Identifier("myVar".to_string()));
        assert_eq!(tokens[1].token, Token::Identifier("userName".to_string()));
        assert_eq!(tokens[2].token, Token::Identifier("file_path".to_string()));
    }

    #[test]
    fn test_tokenize_comment() {
        let input = "# this is a comment\nprint \"hello\"";
        let tokens = tokenize(input).unwrap();
        
        assert!(matches!(tokens[0].token, Token::Comment(_)));
        assert_eq!(tokens[1].token, Token::Newline);
        assert_eq!(tokens[2].token, Token::Print);
    }

    #[test]
    fn test_tokenize_multiline() {
        let input = "print \"hello\"\nprint \"world\"";
        let tokens = tokenize(input).unwrap();
        
        assert_eq!(tokens[0].token, Token::Print);
        assert_eq!(tokens[2].token, Token::Newline);
        assert_eq!(tokens[3].token, Token::Print);
    }

    #[test]
    fn test_tokenize_dot_access() {
        let input = "git.files.staged";
        let tokens = tokenize(input).unwrap();
        
        assert_eq!(tokens[0].token, Token::Identifier("git".to_string()));
        assert_eq!(tokens[1].token, Token::Dot);
        assert_eq!(tokens[2].token, Token::Identifier("files".to_string()));
        assert_eq!(tokens[3].token, Token::Dot);
        assert_eq!(tokens[4].token, Token::Identifier("staged".to_string()));
    }

    #[test]
    fn test_error_unterminated_string() {
        let input = "\"hello\\";
        let result = tokenize(input);
        
        if result.is_err() {
            assert!(matches!(result.unwrap_err(), LexError::UnterminatedString { .. }));
        }
    }

    #[test]
    fn test_error_invalid_character() {
        let input = "print ^";
        let result = tokenize(input);
        
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LexError::UnexpectedChar { .. }));
    }

    #[test]
    fn test_span_tracking() {
        let input = "print \"hello\"";
        let tokens = tokenize(input).unwrap();
        
        assert_eq!(tokens[0].span.line, 1);
        assert_eq!(tokens[0].span.col, 1);
        
        assert_eq!(tokens[1].span.line, 1);
        assert_eq!(tokens[1].span.col, 7);
    }

    #[test]
    fn test_boolean_literals() {
        let input = "true false";
        let tokens = tokenize(input).unwrap();
        
        assert_eq!(tokens[0].token, Token::True);
        assert_eq!(tokens[1].token, Token::False);
    }

    #[test]
    fn test_logical_operators() {
        let input = "and or not";
        let tokens = tokenize(input).unwrap();
        
        assert_eq!(tokens[0].token, Token::And);
        assert_eq!(tokens[1].token, Token::Or);
        assert_eq!(tokens[2].token, Token::Not);
    }

    #[test]
    fn test_arrow_operators() {
        let input = "-> =>";
        let tokens = tokenize(input).unwrap();
        
        assert_eq!(tokens[0].token, Token::Arrow);
        assert_eq!(tokens[1].token, Token::FatArrow);
    }

    #[test]
    fn test_special_chars() {
        let input = "@ $ : ,";
        let tokens = tokenize(input).unwrap();
        
        assert_eq!(tokens[0].token, Token::At);
        assert_eq!(tokens[1].token, Token::Dollar);
        assert_eq!(tokens[2].token, Token::Colon);
        assert_eq!(tokens[3].token, Token::Comma);
    }
}
