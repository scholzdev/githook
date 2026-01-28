use crate::error::{Span, LexError};
use anyhow::Result;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Run,
    Block,
    Group,
    Severity,
    Enabled,
    Require,
    Allow,
    File,
    Parallel,
    Let,
    StagedFiles,
    AllFiles,
    BranchName,
    CommitMessage,
    AuthorEmail,
    AuthorSet,
    AuthorEmailSet,
    AuthorMissing,
    ModifiedLines,
    FilesChanged,
    Additions,
    Deletions,
    CommitsAhead,
    FileExists,
    FileSize,
    Content,
    StagedContent,
    Diff,
    BeStaged,
    Foreach,
    Matches,
    Matching,
    Must,
    Match,
    Contain,
    Contains,
    BlockIf,
    ContainsSecrets,
    WarnIf,
    Greater,
    GreaterOrEqual,
    Less,
    LessOrEqual,
    Equals,
    Message,
    With,
    Interactive,
    Ask,
    Env,
    True,
    False,
    Identifier(String),
    String(String),
    Number(f64),
    At,
    In,
    MacroName(String),
    Not,
    And,
    Or,
    Comma,
    Where,
    When,
    Else,
    Use,
    Import,
    Macro,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    LeftParen,
    RightParen,
    Newline,
    DoubleEquals,
    Colon,
    Slash,
    DoubleQuote,
    Arrow,
    NotEquals,
    Comment(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Span,
}

pub fn tokenize_with_spans(input: &str) -> Result<Vec<SpannedToken>, LexError> {
    let mut tokens = Vec::with_capacity(input.len() / 4);
    let mut chars = input.chars().peekable();

    let mut line: usize = 1;
    let mut col: usize = 1;
    let mut offset: usize = 0;

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
                    span: Span::new(start_line, start_col, start_offset, offset) 
                });
            }
            '{' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                tokens.push(SpannedToken { 
                    token: Token::LeftBrace, 
                    span: Span::new(start_line, start_col, start_offset, offset) 
                });
            }
            '}' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                tokens.push(SpannedToken { 
                    token: Token::RightBrace, 
                    span: Span::new(start_line, start_col, start_offset, offset) 
                });
            }
            ')' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                tokens.push(SpannedToken { 
                    token: Token::RightParen, 
                    span: Span::new(start_line, start_col, start_offset, offset) 
                });
            }
            '(' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                tokens.push(SpannedToken { 
                    token: Token::LeftParen, 
                    span: Span::new(start_line, start_col, start_offset, offset) 
                });
            }
            '[' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                tokens.push(SpannedToken { 
                    token: Token::LeftBracket, 
                    span: Span::new(start_line, start_col, start_offset, offset) 
                });
            }
            ']' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                tokens.push(SpannedToken { 
                    token: Token::RightBracket, 
                    span: Span::new(start_line, start_col, start_offset, offset) 
                });
            }
            ',' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                tokens.push(SpannedToken { 
                    token: Token::Comma, 
                    span: Span::new(start_line, start_col, start_offset, offset) 
                });
            }
            '>' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                if matches!(chars.peek(), Some('=')) {
                    chars.next();
                    bump('=', &mut line, &mut col, &mut offset);
                    tokens.push(SpannedToken { 
                        token: Token::GreaterOrEqual, 
                        span: Span::new(start_line, start_col, start_offset, offset) 
                    });
                } else {
                    tokens.push(SpannedToken { 
                        token: Token::Greater, 
                        span: Span::new(start_line, start_col, start_offset, offset) 
                    });
                }
            }
            '<' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                if matches!(chars.peek(), Some('=')) {
                    chars.next();
                    bump('=', &mut line, &mut col, &mut offset);
                    tokens.push(SpannedToken { 
                        token: Token::LessOrEqual, 
                        span: Span::new(start_line, start_col, start_offset, offset) 
                    });
                } else {
                    tokens.push(SpannedToken { 
                        token: Token::Less, 
                        span: Span::new(start_line, start_col, start_offset, offset) 
                    });
                }
            }
            '=' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                if matches!(chars.peek(), Some('=')) {
                    chars.next();
                    bump('=', &mut line, &mut col, &mut offset);
                    tokens.push(SpannedToken { 
                        token: Token::DoubleEquals, 
                        span: Span::new(start_line, start_col, start_offset, offset) 
                    });
                } else {
                    tokens.push(SpannedToken { 
                        token: Token::Equals, 
                        span: Span::new(start_line, start_col, start_offset, offset) 
                    });
                }
            }
            ':' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                tokens.push(SpannedToken { 
                    token: Token::Colon, 
                    span: Span::new(start_line, start_col, start_offset, offset) 
                });
            }
            '/' => {
                let slash_line = line;
                let slash_col = col;
                let slash_offset = offset;
                
                chars.next();
                bump('/', &mut line, &mut col, &mut offset);
                
                if let Some(&'*') = chars.peek() {
                    chars.next();
                    bump('*', &mut line, &mut col, &mut offset);
                    
                    let mut comment = String::from("/*");
                    let mut found_end = false;
                    
                    while let Some(&ch) = chars.peek() {
                        chars.next();
                        bump(ch, &mut line, &mut col, &mut offset);
                        comment.push(ch);
                        
                        if ch == '*' {
                            if let Some(&'/') = chars.peek() {
                                chars.next();
                                bump('/', &mut line, &mut col, &mut offset);
                                comment.push('/');
                                found_end = true;
                                break;
                            }
                        }
                    }
                    
                    if !found_end {
                        return Err(LexError::UnterminatedComment { 
                            span: Span::new(slash_line, slash_col, slash_offset, offset)
                        });
                    }
                    
                    tokens.push(SpannedToken {
                        token: Token::Comment(comment),
                        span: Span::new(slash_line, slash_col, slash_offset, offset)
                    });
                } else {
                    return Err(LexError::UnexpectedChar { 
                        ch: '/', 
                        span: Span::new(slash_line, slash_col, slash_offset, offset),
                        suggestion: Some("/*...*/".to_string())
                    });
                }
            }
            '!' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                if matches!(chars.peek(), Some('=')) {
                    chars.next();
                    bump('=', &mut line, &mut col, &mut offset);
                    tokens.push(SpannedToken { 
                        token: Token::NotEquals, 
                        span: Span::new(start_line, start_col, start_offset, offset) 
                    });
                } else {
                    return Err(LexError::UnexpectedChar { 
                        ch: '!', 
                        span: Span::new(start_line, start_col, start_offset, offset),
                        suggestion: Some("!=".to_string())
                    });
                }
            }
            '-' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                if matches!(chars.peek(), Some('>')) {
                    chars.next();
                    bump('>', &mut line, &mut col, &mut offset);
                    tokens.push(SpannedToken { 
                        token: Token::Arrow, 
                        span: Span::new(start_line, start_col, start_offset, offset) 
                    });
                } else {
                    return Err(LexError::UnexpectedChar { 
                        ch: '-', 
                        span: Span::new(start_line, start_col, start_offset, offset),
                        suggestion: Some("->".to_string())
                    });
                }
            }
            '@' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                let mut name = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch.is_alphanumeric() || ch == '_' || ch == ':' {
                        name.push(ch);
                        chars.next();
                        bump(ch, &mut line, &mut col, &mut offset);
                    } else {
                        break;
                    }
                }
                if name.is_empty() {
                    return Err(LexError::UnexpectedEof { 
                        expected: "macro name".to_string(),
                        span: Span::new(start_line, start_col, start_offset, offset)
                    });
                }
                tokens.push(SpannedToken { 
                    token: Token::MacroName(name), 
                    span: Span::new(start_line, start_col, start_offset, offset) 
                });
            }
            '"' => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
                let mut string = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch == '"' {
                        chars.next();
                        bump(ch, &mut line, &mut col, &mut offset);
                        break;
                    }
                    string.push(ch);
                    chars.next();
                    bump(ch, &mut line, &mut col, &mut offset);
                }
                tokens.push(SpannedToken { 
                    token: Token::String(string), 
                    span: Span::new(start_line, start_col, start_offset, offset) 
                });
            }
            '#' => {
                let start_line = line;
                let start_col = col;
                let start_offset = offset;
                let mut comment = String::from("#");
                
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
                    span: Span::new(start_line, start_col, start_offset, offset)
                });
            }
            _ if ch.is_ascii_digit() => {
                let mut number = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch.is_ascii_digit() || ch == '.' {
                        number.push(ch);
                        chars.next();
                        bump(ch, &mut line, &mut col, &mut offset);
                    } else {
                        break;
                    }
                }
                match number.parse::<f64>() {
                    Ok(n) => tokens.push(SpannedToken { 
                        token: Token::Number(n), 
                        span: Span::new(start_line, start_col, start_offset, offset) 
                    }),
                    Err(_) => return Err(LexError::InvalidNumber {
                        text: number,
                        span: Span::new(start_line, start_col, start_offset, offset)
                    })
                }
            }
            _ if ch.is_alphabetic() || ch == '_' => {
                let mut ident = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch.is_alphanumeric() || ch == '_' || ch == '-' {
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
                    "group" => Token::Group,
                    "severity" => Token::Severity,
                    "enabled" => Token::Enabled,
                    "require" => Token::Require,
                    "allow" => Token::Allow,
                    "file" => Token::File,
                    "env" => Token::Env,
                    "parallel" => Token::Parallel,
                    "let" => Token::Let,
                    "staged_files" => Token::StagedFiles,
                    "all_files" => Token::AllFiles,
                    "branch_name" => Token::BranchName,
                    "commit_message" => Token::CommitMessage,
                    "author_email" => Token::AuthorEmail,
                    "author_set" => Token::AuthorSet,
                    "author_email_set" => Token::AuthorEmailSet,
                    "author_missing" => Token::AuthorMissing,
                    "modified_lines" => Token::ModifiedLines,
                    "files_changed" => Token::FilesChanged,
                    "additions" => Token::Additions,
                    "deletions" => Token::Deletions,
                    "commits_ahead" => Token::CommitsAhead,
                    "file_exists" => Token::FileExists,
                    "file_size" => Token::FileSize,
                    "content" => Token::Content,
                    "staged_content" => Token::StagedContent,
                    "diff" => Token::Diff,
                    "be_staged" => Token::BeStaged,
                    "foreach" => Token::Foreach,
                    "true" => Token::True,
                    "false" => Token::False,
                    "where" => Token::Where,
                    "matches" => Token::Matches,
                    "matching" => Token::Matching,
                    "must" => Token::Must,
                    "match" => Token::Match,
                    "contain" => Token::Contain,
                    "contains" => Token::Contains,
                    "block_if" => Token::BlockIf,
                    "warn_if" => Token::WarnIf,
                    "contains_secrets" => Token::ContainsSecrets,
                    "message" => Token::Message,
                    "with" => Token::With,
                    "interactive" => Token::Interactive,
                    "ask" => Token::Ask,
                    "not" => Token::Not,
                    "and" => Token::And,
                    "or" => Token::Or,
                    "in" => Token::In,
                    "when" => Token::When,
                    "else" => Token::Else,
                    "use" => Token::Use,
                    "import" => Token::Import,
                    "macro" => Token::Macro,
                    _ => Token::Identifier(ident),
                };
                tokens.push(SpannedToken { 
                    token, 
                    span: Span::new(start_line, start_col, start_offset, offset) 
                });
            }
            _ => {
                chars.next();
                bump(ch, &mut line, &mut col, &mut offset);
            }
        }
    }

    Ok(tokens)
}