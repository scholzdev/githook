use tower_lsp::lsp_types::*;
use githook_syntax::Statement;

pub fn get_legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: vec![
            SemanticTokenType::KEYWORD,
            SemanticTokenType::FUNCTION,
            SemanticTokenType::STRING,
            SemanticTokenType::NUMBER,
            SemanticTokenType::OPERATOR,
            SemanticTokenType::VARIABLE,
            SemanticTokenType::NAMESPACE,
            SemanticTokenType::PROPERTY,
            SemanticTokenType::COMMENT,
        ],
        token_modifiers: vec![
            SemanticTokenModifier::DECLARATION,
            SemanticTokenModifier::DEFINITION,
        ],
    }
}

pub fn get_semantic_tokens(ast: &Option<Vec<Statement>>, text: &str) -> SemanticTokens {
    let mut tokens = Vec::new();
    
    collect_comments_raw(text, &mut tokens);
    
    if let Some(statements) = ast {
        for stmt in statements {
            collect_tokens_raw(stmt, &mut tokens);
        }
    }
    
    tokens.sort_by(|a, b| {
        a.line.cmp(&b.line).then(a.start.cmp(&b.start))
    });
    
    let mut builder = SemanticTokensBuilder::new();
    for token in tokens {
        builder.push(token.line, token.start, token.length, token.token_type, token.modifiers);
    }
    
    builder.build()
}

#[derive(Debug, Clone)]
struct RawToken {
    line: u32,
    start: u32,
    length: u32,
    token_type: u32,
    modifiers: u32,
}

fn collect_comments_raw(text: &str, tokens: &mut Vec<RawToken>) {
    for (line_idx, line) in text.lines().enumerate() {
        if let Some(pos) = line.find('#') {
            let comment_len = line.len() - pos;
            tokens.push(RawToken {
                line: line_idx as u32,
                start: pos as u32,
                length: comment_len as u32,
                token_type: 8,
                modifiers: 0,
            });
        }
    }
    
    let mut in_comment = false;
    let mut comment_start_line = 0;
    let mut comment_start_col = 0;
    
    for (line_idx, line) in text.lines().enumerate() {
        let mut col = 0;
        let chars: Vec<char> = line.chars().collect();
        
        while col < chars.len() {
            if !in_comment {
                if col + 1 < chars.len() && chars[col] == '/' && chars[col + 1] == '*' {
                    in_comment = true;
                    comment_start_line = line_idx;
                    comment_start_col = col;
                }
            } else if col + 1 < chars.len() && chars[col] == '*' && chars[col + 1] == '/' {
                let end_col = col + 2;
                if comment_start_line == line_idx {
                    let length = end_col - comment_start_col;
                    tokens.push(RawToken {
                        line: line_idx as u32,
                        start: comment_start_col as u32,
                        length: length as u32,
                        token_type: 8,
                        modifiers: 0,
                    });
                } else {
                    for l in comment_start_line..=line_idx {
                        if l == comment_start_line {
                            let line_text = text.lines().nth(l).unwrap_or("");
                            let length = line_text.len() - comment_start_col;
                            tokens.push(RawToken {
                                line: l as u32,
                                start: comment_start_col as u32,
                                length: length as u32,
                                token_type: 8,
                                modifiers: 0,
                            });
                        } else if l == line_idx {
                            tokens.push(RawToken {
                                line: l as u32,
                                start: 0,
                                length: end_col as u32,
                                token_type: 8,
                                modifiers: 0,
                            });
                        } else {
                            let line_len = text.lines().nth(l).map(|s| s.len()).unwrap_or(0);
                            tokens.push(RawToken {
                                line: l as u32,
                                start: 0,
                                length: line_len as u32,
                                token_type: 8,
                                modifiers: 0,
                            });
                        }
                    }
                }
                in_comment = false;
            }
            col += 1;
        }
    }
}

fn collect_tokens_raw(stmt: &Statement, tokens: &mut Vec<RawToken>) {
    match stmt {
        Statement::MacroDef { name, span, body, .. } => {
            tokens.push(RawToken {
                line: (span.line - 1) as u32,
                start: (span.col - 1) as u32,
                length: 5,
                token_type: 0,
                modifiers: 0,
            });
            
            tokens.push(RawToken {
                line: (span.line - 1) as u32,
                start: (span.col + 6) as u32,
                length: name.len() as u32,
                token_type: 1,
                modifiers: 2,
            });
            
            for inner_stmt in body {
                collect_tokens_raw(inner_stmt, tokens);
            }
        }
        Statement::MacroCall { name, namespace, span, .. } => {
            if let Some(ns) = namespace {
                tokens.push(RawToken {
                    line: (span.line - 1) as u32,
                    start: span.col as u32,
                    length: ns.len() as u32,
                    token_type: 6,
                    modifiers: 0,
                });
                tokens.push(RawToken {
                    line: (span.line - 1) as u32,
                    start: (span.col + ns.len() + 1) as u32,
                    length: name.len() as u32,
                    token_type: 1,
                    modifiers: 0,
                });
            } else {
                tokens.push(RawToken {
                    line: (span.line - 1) as u32,
                    start: span.col as u32,
                    length: name.len() as u32,
                    token_type: 1,
                    modifiers: 0,
                });
            }
        }
        Statement::Run { span, .. } => {
            tokens.push(RawToken {
                line: (span.line - 1) as u32,
                start: (span.col - 1) as u32,
                length: 3,
                token_type: 0,
                modifiers: 0,
            });
        }
        Statement::If { then_body, else_body, .. } => {
            for inner_stmt in then_body {
                collect_tokens_raw(inner_stmt, tokens);
            }
            if let Some(else_stmts) = else_body {
                for inner_stmt in else_stmts {
                    collect_tokens_raw(inner_stmt, tokens);
                }
            }
        }
        Statement::ForEach { body, .. } => {
            for inner_stmt in body {
                collect_tokens_raw(inner_stmt, tokens);
            }
        }
        _ => {}
    }
}

struct SemanticTokensBuilder {
    tokens: Vec<SemanticToken>,
    prev_line: u32,
    prev_start: u32,
}

impl SemanticTokensBuilder {
    fn new() -> Self {
        Self {
            tokens: Vec::new(),
            prev_line: 0,
            prev_start: 0,
        }
    }
    
    fn push(&mut self, line: u32, start: u32, length: u32, token_type: u32, token_modifiers: u32) {
        let delta_line = line - self.prev_line;
        let delta_start = if delta_line == 0 {
            start - self.prev_start
        } else {
            start
        };
        
        self.tokens.push(SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type,
            token_modifiers_bitset: token_modifiers,
        });
        
        self.prev_line = line;
        self.prev_start = start;
    }
    
    fn build(self) -> SemanticTokens {
        SemanticTokens {
            result_id: None,
            data: self.tokens,
        }
    }
}
