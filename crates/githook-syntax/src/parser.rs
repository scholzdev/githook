use crate::lexer::{Token, SpannedToken};
use crate::ast::*;
use crate::error::{ParseError, Span};
use anyhow::{Result, bail};

fn peek_token(iter: &mut std::iter::Peekable<std::vec::IntoIter<SpannedToken>>) -> Option<&Token> {
    iter.peek().map(|st| &st.token)
}

fn next_spanned(iter: &mut std::iter::Peekable<std::vec::IntoIter<SpannedToken>>) -> Option<(Token, Span)> {
    iter.next().map(|st| (st.token, st.span))
}

fn next_or_eof(
    iter: &mut std::iter::Peekable<std::vec::IntoIter<SpannedToken>>,
    context: &str,
) -> Result<(Token, Span), ParseError> {
    next_spanned(iter).ok_or_else(|| ParseError::UnexpectedEof {
        expected: "token".to_string(),
        context: Some(context.to_string()),
    })
}

fn skip_newlines_spanned(iter: &mut std::iter::Peekable<std::vec::IntoIter<SpannedToken>>) {
    while matches!(peek_token(iter), Some(Token::Newline) | Some(Token::Comment(_))) {
        iter.next();
    }
}

fn expect_token_spanned(iter: &mut std::iter::Peekable<std::vec::IntoIter<SpannedToken>>, expected: Token) -> Result<Span, ParseError> {
    match next_spanned(iter) {
        Some((token, span)) if token == expected => Ok(span),
        Some((token, span)) => Err(ParseError::UnexpectedToken {
            expected: format!("{:?}", expected),
            found: format!("{:?}", token),
            span,
        }),
        None => Err(ParseError::UnexpectedEof {
            expected: format!("{:?}", expected),
            context: None,
        }),
    }
}

pub fn parse_spanned(tokens: Vec<SpannedToken>) -> Result<Vec<Statement>, ParseError> {
    let mut iter = tokens.into_iter().peekable();
    let capacity = iter.len() / 10;
    let mut statements = Vec::with_capacity(capacity.max(8));

    while iter.peek().is_some() {
        skip_newlines_spanned(&mut iter);
        if iter.peek().is_none() {
            break;
        }
        match parse_statement_spanned(&mut iter) {
            Ok(stmt) => statements.push(stmt),
            Err(e) => {
                let error_span = iter.peek()
                    .map(|st| st.span)
                    .unwrap_or_else(|| Span::new(0, 0, 0, 0));
                return Err(ParseError::InvalidSyntax {
                    message: e.to_string(),
                    span: error_span,
                });
            }
        }
    }

    Ok(statements)
}

fn parse_statement_spanned(iter: &mut std::iter::Peekable<std::vec::IntoIter<SpannedToken>>) -> Result<Statement> {
    let token_peek = peek_token(iter);
    
    match token_peek {
        Some(Token::True) => {
            let (_, span) = next_spanned(iter).unwrap();
            Ok(Statement::BoolLiteral(true, span))
        }
        Some(Token::False) => {
            let (_, span) = next_spanned(iter).unwrap();
            Ok(Statement::BoolLiteral(false, span))
        }
        Some(Token::Run) => {
            let (_, start_span) = next_spanned(iter).unwrap();
            skip_newlines_spanned(iter);
            
            let (cmd, cmd_span) = match next_spanned(iter) {
                Some((Token::String(s), span)) => (s, span),
                Some((tok, span)) => bail!("Expected string after 'run', got {:?} at {:?}", tok, span),
                None => bail!("Expected string after 'run'"),
            };
            
            Ok(Statement::Run(cmd, start_span.merge(&cmd_span)))
        }
        Some(Token::Block) => {
            let (_, start_span) = next_or_eof(iter, "'block' keyword")?;
            skip_newlines_spanned(iter);
            
            let (msg, msg_span) = match next_spanned(iter) {
                Some((Token::String(s), span)) => (s, span),
                Some((tok, span)) => bail!("Expected string after 'block', got {:?} at {:?}", tok, span),
                None => bail!("Expected string after 'block'"),
            };
            
            Ok(Statement::Block(msg, start_span.merge(&msg_span)))
        }
        Some(Token::Allow) => {
            let (_, start_span) = next_or_eof(iter, "'allow' keyword")?;
            skip_newlines_spanned(iter);
            
            let (cmd, cmd_span) = match next_spanned(iter) {
                Some((Token::String(s), span)) => (s, span),
                Some((tok, span)) => bail!("Expected string after 'allow', got {:?} at {:?}", tok, span),
                None => bail!("Expected string after 'allow'"),
            };
            
            Ok(Statement::AllowCommand(cmd, start_span.merge(&cmd_span)))
        }
        Some(Token::Match) => parse_match_spanned(iter),
        Some(Token::Use) => parse_use_spanned(iter),
        Some(Token::Import) => parse_import_spanned(iter),
        Some(Token::Let) => parse_let_spanned(iter),
        Some(Token::String(_)) => parse_file_rule_spanned(iter),
        Some(Token::When) => parse_when_spanned(iter),
        Some(Token::Foreach) => parse_foreach_spanned(iter),
        Some(Token::Parallel) => parse_parallel_spanned(iter),
        Some(Token::Group) => parse_group_spanned(iter),
        Some(Token::Macro) => parse_macro_definition_spanned(iter),
        Some(Token::WarnIf) => parse_conditional_rule_spanned(iter, false),
        Some(Token::BlockIf) => parse_conditional_rule_spanned(iter, true),
        Some(Token::MacroName(_)) => {
            let is_definition = {
                let mut count = 0;
                let mut is_def = false;
                for st in iter.clone().skip(1) {
                    if matches!(st.token, Token::Newline) {
                        count += 1;
                        if count > 10 { break; }
                        continue;
                    }
                    is_def = matches!(st.token, Token::LeftBrace);
                    break;
                }
                is_def
            };
            
            if is_definition {
                parse_macro_definition_spanned(iter)
            } else {
                parse_macro_call_spanned(iter)
            }
        }
        Some(token) => {
            if let Token::Identifier(name) = token {
                let suggestion = suggest_keyword(&name);
                let msg = if let Some(suggested) = suggestion {
                    format!("Unknown keyword '{}'. Did you mean '{}'?", name, suggested)
                } else {
                    format!("Unknown keyword '{}'. Expected: run, when, match, foreach, block, etc.", name)
                };
                return Err(anyhow::anyhow!(msg));
            }
            
            Err(anyhow::anyhow!("Unexpected token {:?}", token))
        }
        None => Err(anyhow::anyhow!("Unexpected end of input while parsing statement"))
    }
}

fn suggest_keyword(input: &str) -> Option<&'static str> {
    const KEYWORDS: &[&str] = &[
        "run", "block", "allow", "when", "match", "foreach", 
        "parallel", "group", "macro", "use", "import", "let",
        "warn_if", "block_if", "true", "false", "else", "matching",
        "message", "contains", "matches", "in", "and", "or", "not"
    ];
    
    let input_lower = input.to_lowercase();
    let mut best_match: Option<(&str, usize)> = None;
    
    for &keyword in KEYWORDS {
        let distance = levenshtein_distance(&input_lower, keyword);
        if distance <= 2 {
            if let Some((_, best_dist)) = best_match {
                if distance < best_dist {
                    best_match = Some((keyword, distance));
                }
            } else {
                best_match = Some((keyword, distance));
            }
        }
    }
    
    best_match.map(|(kw, _)| kw)
}

fn levenshtein_distance(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let a_len = a_chars.len();
    let b_len = b_chars.len();
    
    if a_len == 0 { return b_len; }
    if b_len == 0 { return a_len; }
    
    let mut matrix = vec![vec![0; b_len + 1]; a_len + 1];
    
    for i in 0..=a_len {
        matrix[i][0] = i;
    }
    for j in 0..=b_len {
        matrix[0][j] = j;
    }
    
    for i in 1..=a_len {
        for j in 1..=b_len {
            let cost = if a_chars[i - 1] == b_chars[j - 1] { 0 } else { 1 };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }
    
    matrix[a_len][b_len]
}

fn parse_match_spanned(iter: &mut std::iter::Peekable<std::vec::IntoIter<SpannedToken>>) -> Result<Statement> {
    let (_, start_span) = next_spanned(iter).unwrap();
    skip_newlines_spanned(iter);

    let (subject, subject_span) = match next_spanned(iter) {
        Some((Token::File, span)) => (MatchSubject::File(span), span),
        Some((Token::Content, span)) => (MatchSubject::Content(span), span),
        Some((Token::Diff, span)) => (MatchSubject::Diff(span), span),
        Some((tok, span)) => bail!("Expected 'file', 'content', or 'diff' after 'match', got {:?} at {:?}", tok, span),
        None => bail!("Expected 'file', 'content', or 'diff' after 'match'"),
    };

    skip_newlines_spanned(iter);
    let _ = expect_token_spanned(iter, Token::LeftBrace)?;
    skip_newlines_spanned(iter);

    let mut arms = Vec::with_capacity(8);

    while !matches!(peek_token(iter), Some(Token::RightBrace)) {
        skip_newlines_spanned(iter);
        
        if matches!(peek_token(iter), Some(Token::RightBrace)) {
            break;
        }

        let (pattern, pattern_span) = match peek_token(iter) {
            Some(Token::String(_)) => {
                let (tok, span) = next_spanned(iter).unwrap();
                if let Token::String(s) = tok {
                    (MatchPattern::Wildcard(s, span), span)
                } else {
                    unreachable!()
                }
            }
            Some(Token::Contains) => {
                let (_, kw_span) = next_spanned(iter).unwrap();
                let (text, text_span) = match next_spanned(iter) {
                    Some((Token::String(s), span)) => (s, span),
                    Some((tok, span)) => bail!("Expected string after 'contains', got {:?} at {:?}", tok, span),
                    None => bail!("Expected string after 'contains'"),
                };
                (MatchPattern::Contains(text, kw_span.merge(&text_span)), kw_span.merge(&text_span))
            }
            Some(Token::Matches) => {
                let (_, kw_span) = next_spanned(iter).unwrap();
                let (regex, regex_span) = match next_spanned(iter) {
                    Some((Token::String(s), span)) => (s, span),
                    Some((tok, span)) => bail!("Expected string after 'matches', got {:?} at {:?}", tok, span),
                    None => bail!("Expected string after 'matches'"),
                };
                (MatchPattern::Matches(regex, kw_span.merge(&regex_span)), kw_span.merge(&regex_span))
            }
            Some(Token::Greater) => {
                let (_, kw_span) = next_spanned(iter).unwrap();
                let (value, value_span) = match next_spanned(iter) {
                    Some((Token::Number(n), span)) => (n, span),
                    Some((tok, span)) => bail!("Expected number after '>', got {:?} at {:?}", tok, span),
                    None => bail!("Expected number after '>'"),
                };
                (MatchPattern::GreaterThan(value, kw_span.merge(&value_span)), kw_span.merge(&value_span))
            }
            Some(Token::Less) => {
                let (_, kw_span) = next_spanned(iter).unwrap();
                let (value, value_span) = match next_spanned(iter) {
                    Some((Token::Number(n), span)) => (n, span),
                    Some((tok, span)) => bail!("Expected number after '<', got {:?} at {:?}", tok, span),
                    None => bail!("Expected number after '<'"),
                };
                (MatchPattern::LessThan(value, kw_span.merge(&value_span)), kw_span.merge(&value_span))
            }
            Some(tok) => bail!("Expected pattern in match arm, got {:?}", tok),
            None => bail!("Expected pattern in match arm"),
        };

        skip_newlines_spanned(iter);
        let _ = expect_token_spanned(iter, Token::Arrow)?;
        skip_newlines_spanned(iter);

        let action_stmt = parse_statement_spanned(iter)?;
        let action_span = action_stmt.span();
        let action = vec![action_stmt];

        let arm_span = pattern_span.merge(&action_span);
        arms.push(MatchArm { pattern, action, span: arm_span });
        skip_newlines_spanned(iter);
    }

    let end_span = expect_token_spanned(iter, Token::RightBrace)?;
    let full_span = start_span.merge(&subject_span).merge(&end_span);

    Ok(Statement::Match { subject, arms, span: full_span })
}

fn parse_use_spanned(iter: &mut std::iter::Peekable<std::vec::IntoIter<SpannedToken>>) -> Result<Statement> {
    let (_, start_span) = next_spanned(iter).unwrap();
    skip_newlines_spanned(iter);

    let (package_spec, spec_span) = match next_spanned(iter) {
        Some((Token::String(s), span)) => (s, span),
        Some((tok, span)) => bail!("Expected package specifier string after 'use', got {:?} at {:?}", tok, span),
        None => bail!("Expected package specifier string after 'use'"),
    };

    if !package_spec.starts_with('@') {
        bail!("Package specifier must start with '@', got: {}", package_spec);
    }

    let rest = &package_spec[1..];
    let parts: Vec<&str> = rest.split('/').collect();
    if parts.len() != 2 {
        bail!("Invalid package specifier format, expected @namespace/name, got: {}", package_spec);
    }

    let namespace = parts[0].to_string();
    let name = parts[1].to_string();

    let mut end_span = spec_span;
    let alias = if matches!(peek_token(iter), Some(Token::Identifier(id)) if id == "as") {
        next_spanned(iter);
        skip_newlines_spanned(iter);
        match next_spanned(iter) {
            Some((Token::Identifier(alias_name), span)) => {
                end_span = span;
                Some(alias_name)
            }
            Some((tok, span)) => bail!("Expected identifier after 'as', got {:?} at {:?}", tok, span),
            None => bail!("Expected identifier after 'as'"),
        }
    } else {
        None
    };

    Ok(Statement::Use {
        namespace,
        name,
        alias,
        span: start_span.merge(&end_span),
    })
}

fn parse_import_spanned(iter: &mut std::iter::Peekable<std::vec::IntoIter<SpannedToken>>) -> Result<Statement> {
    let (_, start_span) = next_spanned(iter).unwrap();
    skip_newlines_spanned(iter);

    let (path, path_span) = match next_spanned(iter) {
        Some((Token::String(s), span)) => (s, span),
        Some((tok, span)) => bail!("Expected file path string after 'import', got {:?} at {:?}", tok, span),
        None => bail!("Expected file path string after 'import'"),
    };

    let mut end_span = path_span;
    let alias = if matches!(peek_token(iter), Some(Token::Identifier(id)) if id == "as") {
        next_spanned(iter);
        skip_newlines_spanned(iter);
        match next_spanned(iter) {
            Some((Token::Identifier(alias_name), span)) => {
                end_span = span;
                Some(alias_name)
            }
            Some((tok, span)) => bail!("Expected identifier after 'as', got {:?} at {:?}", tok, span),
            None => bail!("Expected identifier after 'as'"),
        }
    } else {
        None
    };

    Ok(Statement::Import { 
        path, 
        alias, 
        span: start_span.merge(&end_span),
    })
}

fn parse_let_spanned(iter: &mut std::iter::Peekable<std::vec::IntoIter<SpannedToken>>) -> Result<Statement> {
    let (_, start_span) = next_spanned(iter).unwrap();

    let (name, _) = match next_spanned(iter) {
        Some((Token::Identifier(id), span)) => (id, span),
        Some((tok, span)) => bail!("Expected identifier after 'let', got {:?} at {:?}", tok, span),
        None => bail!("Expected identifier after 'let'"),
    };

    let _ = expect_token_spanned(iter, Token::Equals)?;
    let _ = expect_token_spanned(iter, Token::LeftBracket)?;

    let mut items = Vec::new();

    loop {
        skip_newlines_spanned(iter);
        
        match peek_token(iter) {
            Some(Token::RightBracket) => {
                break;
            }
            Some(Token::String(_)) => {
                let (s, _) = match next_spanned(iter) {
                    Some((Token::String(s), span)) => (s, span),
                    _ => unreachable!(),
                };
                items.push(s);

                skip_newlines_spanned(iter);

                match peek_token(iter) {
                    Some(Token::Comma) => {
                        next_spanned(iter);
                    }
                    Some(Token::RightBracket) => {}
                    Some(tok) => bail!("Expected ',' or ']' in list, got {:?}", tok),
                    None => bail!("Expected ']' to close list"),
                }
            }
            Some(tok) => bail!("Expected string or ']' in list, got {:?}", tok),
            None => bail!("Unexpected end of input in list literal"),
        }
    }

    let end_span = expect_token_spanned(iter, Token::RightBracket)?;

    Ok(Statement::LetStringList { 
        name, 
        items, 
        span: start_span.merge(&end_span),
    })
}

fn parse_file_rule_spanned(iter: &mut std::iter::Peekable<std::vec::IntoIter<SpannedToken>>) -> Result<Statement> {
    let (pattern, start_span) = match next_spanned(iter) {
        Some((Token::String(s), span)) => (s, span),
        _ => unreachable!(),
    };
    
    let _ = expect_token_spanned(iter, Token::Must)?;

    let is_negated = match peek_token(iter) {
        Some(Token::Not) => {
            next_spanned(iter);
            true
        }
        _ => false,
    };

    match next_spanned(iter) {
        Some((Token::Identifier(ref s), _)) if s == "be" => {}
        Some((tok, span)) => bail!("Expected 'be' after 'must{}', got {:?} at {:?}", if is_negated { " not" } else { "" }, tok, span),
        None => bail!("Expected 'be' after 'must{}'", if is_negated { " not" } else { "" }),
    }

    let end_span = match next_spanned(iter) {
        Some((Token::Identifier(ref s), span)) if s == "staged" => span,
        Some((tok, span)) => bail!("Expected 'staged' after 'be', got {:?} at {:?}", tok, span),
        None => bail!("Expected 'staged' after 'be'"),
    };

    Ok(Statement::FileRule {
        pattern,
        must_be_staged: !is_negated,
        span: start_span.merge(&end_span),
    })
}

fn parse_body_spanned(iter: &mut std::iter::Peekable<std::vec::IntoIter<SpannedToken>>) -> Result<Vec<Statement>> {
    let mut statements = Vec::new();
    loop {
        skip_newlines_spanned(iter);
        if matches!(peek_token(iter), Some(Token::RightBrace)) || iter.peek().is_none() {
            break;
        }
        statements.push(parse_statement_spanned(iter)?);
    }
    Ok(statements)
}

fn parse_condition_spanned(iter: &mut std::iter::Peekable<std::vec::IntoIter<SpannedToken>>) -> Result<BlockCondition> {
    parse_or_condition_spanned(iter)
}

fn parse_or_condition_spanned(iter: &mut std::iter::Peekable<std::vec::IntoIter<SpannedToken>>) -> Result<BlockCondition> {
    let mut left = parse_and_condition_spanned(iter)?;
    loop {
        skip_newlines_spanned(iter);
        if matches!(peek_token(iter), Some(Token::Or)) {
            let (_, or_span) = next_spanned(iter).unwrap();
            let right = parse_and_condition_spanned(iter)?;
            let right_span = match &right {
                BlockCondition::Comparison { span, .. } => *span,
                BlockCondition::Bool(_, span) => *span,
                BlockCondition::And { span, .. } => *span,
                BlockCondition::Or { span, .. } => *span,
                BlockCondition::Not { span, .. } => *span,
                _ => or_span,
            };
            left = BlockCondition::Or { 
                left: Box::new(left), 
                right: Box::new(right), 
                span: or_span.merge(&right_span),
            };
        } else {
            break;
        }
    }
    Ok(left)
}

fn parse_and_condition_spanned(iter: &mut std::iter::Peekable<std::vec::IntoIter<SpannedToken>>) -> Result<BlockCondition> {
    let mut left = parse_atom_condition_spanned(iter)?;
    loop {
        skip_newlines_spanned(iter);
        if matches!(peek_token(iter), Some(Token::And)) {
            let (_, and_span) = next_spanned(iter).unwrap();
            let right = parse_atom_condition_spanned(iter)?;
            let right_span = match &right {
                BlockCondition::Comparison { span, .. } => *span,
                BlockCondition::Bool(_, span) => *span,
                BlockCondition::And { span, .. } => *span,
                BlockCondition::Or { span, .. } => *span,
                BlockCondition::Not { span, .. } => *span,
                _ => and_span,
            };
            left = BlockCondition::And { 
                left: Box::new(left), 
                right: Box::new(right), 
                span: and_span.merge(&right_span),
            };
        } else {
            break;
        }
    }
    Ok(left)
}

fn parse_atom_condition_spanned(iter: &mut std::iter::Peekable<std::vec::IntoIter<SpannedToken>>) -> Result<BlockCondition> {
    skip_newlines_spanned(iter);
    
    let is_negated = if matches!(peek_token(iter), Some(Token::Not)) {
        next_spanned(iter);
        skip_newlines_spanned(iter);
        true
    } else {
        false
    };
    
    if matches!(peek_token(iter), Some(Token::LeftParen)) {
        next_spanned(iter);
        let cond = parse_or_condition_spanned(iter)?;
        expect_token_spanned(iter, Token::RightParen)?;
        
        return if is_negated {
            let span = match &cond {
                BlockCondition::Comparison { span, .. } => *span,
                BlockCondition::Bool(_, span) => *span,
                BlockCondition::And { span, .. } => *span,
                BlockCondition::Or { span, .. } => *span,
                BlockCondition::Not { span, .. } => *span,
                _ => cond.span(),
            };
            Ok(BlockCondition::Not { inner: Box::new(cond), span })
        } else {
            Ok(cond)
        };
    }
    
    if let Some(mut cond) = try_parse_unified_comparison_spanned(iter)? {
        if is_negated {
            if let BlockCondition::Comparison { left, operator, right, span, .. } = cond {
                cond = BlockCondition::Comparison {
                    left,
                    operator,
                    right,
                    negated: true,
                    span,
                };
            } else {
                let span = match &cond {
                    BlockCondition::Comparison { span, .. } => *span,
                    _ => cond.span(),
                };
                cond = BlockCondition::Not { inner: Box::new(cond), span };
            }
        }
        return Ok(cond);
    }
    
    let result = match peek_token(iter) {
        Some(Token::True) => {
            let (_, span) = next_spanned(iter).unwrap();
            Ok(BlockCondition::Bool(true, span))
        }
        Some(Token::False) => {
            let (_, span) = next_spanned(iter).unwrap();
            Ok(BlockCondition::Bool(false, span))
        }
        Some(Token::ContainsSecrets) => {
            let (_, span) = next_spanned(iter).unwrap();
            Ok(BlockCondition::ContainsSecrets(span))
        }
        Some(Token::AuthorSet) => {
            let (_, span) = next_spanned(iter).unwrap();
            Ok(BlockCondition::AuthorSet(span))
        }
        Some(Token::AuthorEmailSet) => {
            let (_, span) = next_spanned(iter).unwrap();
            Ok(BlockCondition::AuthorEmailSet(span))
        }
        Some(Token::AuthorMissing) => {
            let (_, span) = next_spanned(iter).unwrap();
            Ok(BlockCondition::AuthorMissing(span))
        }
        Some(Token::Env) => {
            let (_, start_span) = next_spanned(iter).unwrap();
            let (key, _) = match next_spanned(iter) {
                Some((Token::String(s), span)) => (s, span),
                Some((tok, span)) => bail!("Expected environment variable name, got {:?} at {:?}", tok, span),
                None => bail!("Expected environment variable name"),
            };
            let _ = expect_token_spanned(iter, Token::Equals)?;
            let (value, end_span) = match next_spanned(iter) {
                Some((Token::String(s), span)) => (s, span),
                Some((tok, span)) => bail!("Expected value after '=', got {:?} at {:?}", tok, span),
                None => bail!("Expected value after '='"),
            };
            Ok(BlockCondition::EnvEquals(key, value, start_span.merge(&end_span)))
        }
        _ => Err(anyhow::anyhow!("Expected condition, got {:?}", peek_token(iter))),
    };
    
    if is_negated {
        let result_cond = result?;
        let span = match &result_cond {
            BlockCondition::Comparison { span, .. } => *span,
            BlockCondition::Bool(_, span) => *span,
            BlockCondition::ContainsSecrets(span) => *span,
            BlockCondition::AuthorSet(span) => *span,
            BlockCondition::AuthorEmailSet(span) => *span,
            BlockCondition::AuthorMissing(span) => *span,
            BlockCondition::EnvEquals(_, _, span) => *span,
            _ => result_cond.span(),
        };
        Ok(BlockCondition::Not { inner: Box::new(result_cond), span })
    } else {
        result
    }
}

fn try_parse_unified_comparison_spanned(iter: &mut std::iter::Peekable<std::vec::IntoIter<SpannedToken>>) -> Result<Option<BlockCondition>> {
    let start_pos = iter.clone();
    
    let (property_token, property_span) = match next_spanned(iter) {
        Some((token, span)) => (token, span),
        None => return Ok(None),
    };
    
    let property_value = match property_token {
        Token::Content => PropertyValue::Content(property_span),
        Token::BranchName => PropertyValue::BranchName(property_span),
        Token::CommitMessage => PropertyValue::CommitMessage(property_span),
        Token::AuthorEmail => PropertyValue::Placeholder("author_email".to_string(), property_span),
        Token::ModifiedLines => PropertyValue::ModifiedLines(property_span),
        Token::FilesChanged => PropertyValue::FilesChanged(property_span),
        Token::Additions => PropertyValue::Additions(property_span),
        Token::Deletions => PropertyValue::Deletions(property_span),
        Token::CommitsAhead => PropertyValue::CommitsAhead(property_span),
        Token::FileExists => PropertyValue::Placeholder("file_exists".to_string(), property_span),
        Token::FileSize => PropertyValue::FileSize(property_span),
        Token::Diff => PropertyValue::Diff(property_span),
        _ => {
            *iter = start_pos;
            return Ok(None);
        }
    };
    
    skip_newlines_spanned(iter);
    
    let (op_token, _op_span) = match next_spanned(iter) {
        Some((token, span)) => (token, span),
        None => {
            *iter = start_pos;
            return Ok(None);
        }
    };
    
    skip_newlines_spanned(iter);
    
    let (operator, comparison_value, end_span) = match op_token {
        Token::Match => {
            let (pattern, pattern_span) = match next_spanned(iter) {
                Some((Token::String(s), span)) => (s, span),
                _ => {
                    *iter = start_pos;
                    return Ok(None);
                }
            };
            (ComparisonOperator::Matches, ComparisonValue::String(pattern, pattern_span), pattern_span)
        }
        
        Token::Matches => {
            let (regex, regex_span) = match next_spanned(iter) {
                Some((Token::String(s), span)) => (s, span),
                _ => {
                    *iter = start_pos;
                    return Ok(None);
                }
            };
            (ComparisonOperator::Matches, ComparisonValue::String(regex, regex_span), regex_span)
        }
        
        Token::Greater => {
            let (value, value_span) = match next_spanned(iter) {
                Some((Token::Number(n), span)) => (n, span),
                _ => {
                    *iter = start_pos;
                    return Ok(None);
                }
            };
            (ComparisonOperator::Greater, ComparisonValue::Number(value, value_span), value_span)
        }
        
        Token::GreaterOrEqual => {
            let (value, value_span) = match next_spanned(iter) {
                Some((Token::Number(n), span)) => (n, span),
                _ => {
                    *iter = start_pos;
                    return Ok(None);
                }
            };
            (ComparisonOperator::GreaterOrEqual, ComparisonValue::Number(value, value_span), value_span)
        }
        
        Token::Less => {
            let (value, value_span) = match next_spanned(iter) {
                Some((Token::Number(n), span)) => (n, span),
                _ => {
                    *iter = start_pos;
                    return Ok(None);
                }
            };
            (ComparisonOperator::Less, ComparisonValue::Number(value, value_span), value_span)
        }
        
        Token::LessOrEqual => {
            let (value, value_span) = match next_spanned(iter) {
                Some((Token::Number(n), span)) => (n, span),
                _ => {
                    *iter = start_pos;
                    return Ok(None);
                }
            };
            (ComparisonOperator::LessOrEqual, ComparisonValue::Number(value, value_span), value_span)
        }
        
        Token::DoubleEquals => {
            let (value, value_span) = match next_spanned(iter) {
                Some((Token::String(s), span)) => (ComparisonValue::String(s, span), span),
                Some((Token::Number(n), span)) => (ComparisonValue::Number(n, span), span),
                Some((Token::Identifier(id), span)) => (ComparisonValue::Identifier(id, span), span),
                _ => {
                    *iter = start_pos;
                    return Ok(None);
                }
            };
            (ComparisonOperator::Equals, value, value_span)
        }
        
        Token::Contain => {
            let (text, text_span) = match next_spanned(iter) {
                Some((Token::String(s), span)) => (s, span),
                _ => {
                    *iter = start_pos;
                    return Ok(None);
                }
            };
            (ComparisonOperator::Contains, ComparisonValue::String(text, text_span), text_span)
        }
        
        _ => {
            *iter = start_pos;
            return Ok(None);
        }
    };
    
    let full_span = property_span.merge(&end_span);
    
    Ok(Some(BlockCondition::Comparison {
        left: property_value,
        operator,
        right: comparison_value,
        negated: false,
        span: full_span,
    }))
}

fn parse_when_spanned(iter: &mut std::iter::Peekable<std::vec::IntoIter<SpannedToken>>) -> Result<Statement> {
    let (_, start_span) = next_spanned(iter).unwrap();
    let condition = parse_condition_spanned(iter)?;

    skip_newlines_spanned(iter);
    let _ = expect_token_spanned(iter, Token::LeftBrace)?;

    let body = parse_body_spanned(iter)?;
    skip_newlines_spanned(iter);
    let mut end_span = expect_token_spanned(iter, Token::RightBrace)?;

    let is_else = matches!(peek_token(iter), Some(Token::Else));
    let else_body = if is_else {
        next_spanned(iter);
        skip_newlines_spanned(iter);
        let _ = expect_token_spanned(iter, Token::LeftBrace)?;

        let else_body = parse_body_spanned(iter)?;
        skip_newlines_spanned(iter);
        end_span = expect_token_spanned(iter, Token::RightBrace)?;

        Some(else_body)
    } else {
        None
    };

    Ok(Statement::When {
        condition,
        body,
        else_body,
        span: start_span.merge(&end_span),
    })
}

fn parse_foreach_spanned(iter: &mut std::iter::Peekable<std::vec::IntoIter<SpannedToken>>) -> Result<Statement> {
    let (_, start_span) = next_spanned(iter).unwrap();
    
    let (var, _) = match next_spanned(iter) {
        Some((Token::Identifier(id), span)) => (id, span),
        Some((Token::File, _)) => ("file".to_string(), start_span),
        Some((tok, span)) => bail!("Expected identifier after 'foreach', got {:?} at {:?}", tok, span),
        None => bail!("Expected identifier after 'foreach'"),
    };
    
    match next_spanned(iter) {
        Some((Token::In, _)) => {}
        Some((tok, span)) => bail!("Expected 'in' after 'foreach', got {:?} at {:?}", tok, span),
        None => bail!("Expected 'in' after 'foreach'"),
    }

    match peek_token(iter) {
        Some(Token::LeftBracket) => {
            next_spanned(iter);
            let mut items = Vec::new();
            
            loop {
                skip_newlines_spanned(iter);
                
                match peek_token(iter) {
                    Some(Token::RightBracket) => {
                        next_spanned(iter);
                        break;
                    }
                    Some(Token::String(_)) => {
                        if let Some((Token::String(s), span)) = next_spanned(iter) {
                            items.push(Argument::String(s, span));
                        }
                    }
                    Some(Token::Number(_)) => {
                        if let Some((Token::Number(n), span)) = next_spanned(iter) {
                            items.push(Argument::Number(n, span));
                        }
                    }
                    Some(Token::Identifier(_)) => {
                        if let Some((Token::Identifier(id), span)) = next_spanned(iter) {
                            items.push(Argument::Identifier(id, span));
                        }
                    }
                    Some(tok) => bail!("Expected array item or ']', got {:?}", tok),
                    None => bail!("Expected ']' to close array"),
                }
                
                skip_newlines_spanned(iter);
                
                match peek_token(iter) {
                    Some(Token::Comma) => {
                        next_spanned(iter);
                    }
                    Some(Token::RightBracket) => {}
                    Some(tok) => bail!("Expected ',' or ']' in array, got {:?}", tok),
                    None => bail!("Expected ']' to close array"),
                }
            }
            
            skip_newlines_spanned(iter);
            let _ = expect_token_spanned(iter, Token::LeftBrace)?;
            let body = parse_body_spanned(iter)?;
            let end_span = expect_token_spanned(iter, Token::RightBrace)?;
            
            Ok(Statement::ForEachArray {
                var,
                items,
                body,
                span: start_span.merge(&end_span),
            })
        }
        
        Some(Token::LeftBrace) => {
            next_spanned(iter);

            let (list, _) = match next_spanned(iter) {
                Some((Token::Identifier(id), span)) => (id, span),
                Some((tok, span)) => bail!("Expected list identifier inside '{{}}', got {:?} at {:?}", tok, span),
                None => bail!("Expected list identifier inside '{{}}'"),
            };

            let _ = expect_token_spanned(iter, Token::RightBrace)?;
            let _ = expect_token_spanned(iter, Token::LeftBrace)?;
            let body = parse_body_spanned(iter)?;
            let end_span = expect_token_spanned(iter, Token::RightBrace)?;
            
            Ok(Statement::ForEachStringList { 
                var, 
                list, 
                body, 
                span: start_span.merge(&end_span),
            })
        }

        Some(Token::StagedFiles) => {
            next_spanned(iter);

            let pattern = if matches!(peek_token(iter), Some(Token::Matching)) {
                next_spanned(iter);
                match next_spanned(iter) {
                    Some((Token::String(s), _)) => s,
                    Some((tok, span)) => bail!("Expected pattern string after 'matching', got {:?} at {:?}", tok, span),
                    None => bail!("Expected pattern string after 'matching'"),
                }
            } else {
                "*".to_string()
            };

            skip_newlines_spanned(iter);

            let where_cond = if matches!(peek_token(iter), Some(Token::Where)) {
                next_spanned(iter);

                let not_span = if matches!(peek_token(iter), Some(Token::Not)) {
                    Some(next_spanned(iter).unwrap().1)
                } else {
                    None
                };

                let base = parse_condition_spanned(iter)?;
                Some(if let Some(not_s) = not_span {
                    let base_span = match &base {
                        BlockCondition::Comparison { span, .. } => *span,
                        BlockCondition::Bool(_, span) => *span,
                        _ => not_s,
                    };
                    BlockCondition::Not { inner: Box::new(base), span: not_s.merge(&base_span) }
                } else {
                    base
                })
            } else {
                None
            };

            skip_newlines_spanned(iter);
            let _ = expect_token_spanned(iter, Token::LeftBrace)?;
            let body = parse_body_spanned(iter)?;
            let end_span = expect_token_spanned(iter, Token::RightBrace)?;

            Ok(Statement::ForEachStagedFiles { 
                var, 
                pattern, 
                where_cond, 
                body, 
                span: start_span.merge(&end_span),
            })
        }

        Some(tok) => bail!("Expected '{{' or 'staged_files' after 'foreach <var> in', got {:?}", tok),
        None => bail!("Expected '{{' or 'staged_files' after 'foreach <var> in'"),
    }
}

fn parse_parallel_spanned(iter: &mut std::iter::Peekable<std::vec::IntoIter<SpannedToken>>) -> Result<Statement> {
    let (_, start_span) = next_spanned(iter).unwrap();
    let _ = expect_token_spanned(iter, Token::LeftBrace)?;
    let mut commands = Vec::new();

    loop {
        skip_newlines_spanned(iter);
        if matches!(peek_token(iter), Some(Token::RightBrace)) {
            break;
        }

        let cmd = match peek_token(iter) {
            Some(Token::Run) => {
                next_spanned(iter);
                match next_spanned(iter) {
                    Some((Token::String(s), _)) => s,
                    Some((tok, span)) => bail!("Expected string after 'run' in parallel block, got {:?} at {:?}", tok, span),
                    None => bail!("Expected string after 'run' in parallel block"),
                }
            }
            Some(Token::String(_)) => {
                match next_spanned(iter) {
                    Some((Token::String(s), _)) => s,
                    Some((tok, span)) => bail!("Expected command string in parallel block, got {:?} at {:?}", tok, span),
                    None => bail!("Expected command string in parallel block"),
                }
            }
            Some(tok) => bail!("Expected 'run' or command string in parallel block, got {:?}", tok),
            None => bail!("Unexpected end of parallel block"),
        };
        
        commands.push(cmd);
    }

    let end_span = expect_token_spanned(iter, Token::RightBrace)?;
    if commands.is_empty() {
        bail!("Parallel block must contain at least one command");
    }
    
    Ok(Statement::Parallel { 
        commands, 
        span: start_span.merge(&end_span),
    })
}

fn parse_group_spanned(iter: &mut std::iter::Peekable<std::vec::IntoIter<SpannedToken>>) -> Result<Statement> {
    let (_, start_span) = next_spanned(iter).unwrap();
    
    let (name, _) = match next_spanned(iter) {
        Some((Token::Identifier(id), span)) => (id, span),
        Some((tok, span)) => bail!("Expected group name after 'group', got {:?} at {:?}", tok, span),
        None => bail!("Expected group name after 'group'"),
    };

    skip_newlines_spanned(iter);
    let _ = expect_token_spanned(iter, Token::LeftBrace)?;
    skip_newlines_spanned(iter);

    let mut severity: Option<GroupSeverity> = None;
    let mut enabled: Option<bool> = None;
    let mut body: Vec<Statement> = Vec::new();

    loop {
        skip_newlines_spanned(iter);
        
        if matches!(peek_token(iter), Some(Token::RightBrace)) {
            break;
        }

        match peek_token(iter) {
            Some(Token::Severity) => {
                next_spanned(iter);
                skip_newlines_spanned(iter);
                let _ = expect_token_spanned(iter, Token::Colon)?;
                skip_newlines_spanned(iter);
                
                let (sev, _) = match next_spanned(iter) {
                    Some((Token::Identifier(s), span)) if s == "critical" => (GroupSeverity::Critical(span), span),
                    Some((Token::Identifier(s), span)) if s == "warning" => (GroupSeverity::Warning(span), span),
                    Some((Token::Identifier(s), span)) if s == "info" => (GroupSeverity::Info(span), span),
                    Some((tok, span)) => bail!("Expected severity value (critical/warning/info), got {:?} at {:?}", tok, span),
                    None => bail!("Expected severity value"),
                };
                severity = Some(sev);
                skip_newlines_spanned(iter);
            }
            
            Some(Token::Enabled) => {
                next_spanned(iter);
                skip_newlines_spanned(iter);
                let _ = expect_token_spanned(iter, Token::Colon)?;
                skip_newlines_spanned(iter);
                
                let en = match next_spanned(iter) {
                    Some((Token::True, _)) => true,
                    Some((Token::False, _)) => false,
                    Some((tok, span)) => bail!("Expected boolean value (true/false) after 'enabled:', got {:?} at {:?}", tok, span),
                    None => bail!("Expected boolean value after 'enabled:'"),
                };
                enabled = Some(en);
                skip_newlines_spanned(iter);
            }
            
            Some(Token::RightBrace) => {
                break;
            }
            
            _ => {
                body.push(parse_statement_spanned(iter)?);
            }
        }
    }

    let end_span = expect_token_spanned(iter, Token::RightBrace)?;
    let def_span = start_span.merge(&end_span);

    Ok(Statement::Group {
        definition: GroupDefinition {
            name,
            severity,
            enabled,
            body,
            span: def_span,
        },
        span: def_span,
    })
}

fn parse_macro_definition_spanned(iter: &mut std::iter::Peekable<std::vec::IntoIter<SpannedToken>>) -> Result<Statement> {
    let (first_token, start_span) = next_spanned(iter).unwrap();
    skip_newlines_spanned(iter);

    let name = match first_token {
        Token::Macro => {
            match next_spanned(iter) {
                Some((Token::Identifier(id), _)) => id,
                Some((tok, span)) => bail!("Expected macro name after 'macro', got {:?} at {:?}", tok, span),
                None => bail!("Expected macro name after 'macro'"),
            }
        }
        Token::MacroName(name) => {
            name.strip_prefix('@')
                .map(|s| s.to_string())
                .unwrap_or_else(|| name.to_string())
        }
        _ => bail!("Expected 'macro' or '@name'"),
    };

    skip_newlines_spanned(iter);

    let params = if matches!(peek_token(iter), Some(Token::LeftParen)) {
        expect_token_spanned(iter, Token::LeftParen)?;
        let mut params = Vec::new();

        loop {
            skip_newlines_spanned(iter);
            
            match peek_token(iter) {
                Some(Token::RightParen) => {
                    next_spanned(iter);
                    break;
                }
                Some(Token::Identifier(_)) => {
                    if let Some((Token::Identifier(param), _)) = next_spanned(iter) {
                        params.push(param);
                    }
                    
                    skip_newlines_spanned(iter);
                    
                    match peek_token(iter) {
                        Some(Token::Comma) => {
                            next_spanned(iter);
                        }
                        Some(Token::RightParen) => {}
                        Some(tok) => bail!("Expected ',' or ')' in parameter list, got {:?}", tok),
                        None => bail!("Expected ')' to close parameter list"),
                    }
                }
                Some(tok) => bail!("Expected parameter name or ')' in parameter list, got {:?}", tok),
                None => bail!("Expected ')' to close parameter list"),
            }
        }

        params
    } else {
        Vec::new()
    };

    skip_newlines_spanned(iter);
    expect_token_spanned(iter, Token::LeftBrace)?;

    let body = parse_body_spanned(iter)?;
    
    skip_newlines_spanned(iter);
    let end_span = expect_token_spanned(iter, Token::RightBrace)?;

    Ok(Statement::MacroDefinition {
        name,
        params,
        body,
        span: start_span.merge(&end_span),
    })
}

fn parse_macro_call_spanned(iter: &mut std::iter::Peekable<std::vec::IntoIter<SpannedToken>>) -> Result<Statement> {
    let (macro_name_token, start_span) = next_spanned(iter).unwrap();
    
    let macro_name = match macro_name_token {
        Token::MacroName(name) => name,
        _ => bail!("Expected macro name"),
    };

    let (namespace, name) = if macro_name.contains(':') {
        let parts: Vec<&str> = macro_name.trim_start_matches('@').split(':').collect();
        if parts.len() != 2 {
            bail!("Invalid macro name format: {}", macro_name);
        }
        (Some(parts[0].to_string()), parts[1].to_string())
    } else {
        (None, macro_name.trim_start_matches('@').to_string())
    };

    let args = if matches!(peek_token(iter), Some(Token::LeftParen)) {
        expect_token_spanned(iter, Token::LeftParen)?;
        let mut args = Vec::new();

        loop {
            skip_newlines_spanned(iter);
            
            match peek_token(iter) {
                Some(Token::RightParen) => {
                    next_spanned(iter);
                    break;
                }
                Some(Token::String(_)) => {
                    if let Some((Token::String(s), span)) = next_spanned(iter) {
                        args.push(Argument::String(s, span));
                    }
                    
                    skip_newlines_spanned(iter);
                    
                    match peek_token(iter) {
                        Some(Token::Comma) => {
                            next_spanned(iter);
                        }
                        Some(Token::RightParen) => {}
                        Some(tok) => bail!("Expected ',' or ')' in argument list, got {:?}", tok),
                        None => bail!("Expected ')' to close argument list"),
                    }
                }
                Some(Token::Number(_)) => {
                    if let Some((Token::Number(n), span)) = next_spanned(iter) {
                        args.push(Argument::Number(n, span));
                    }
                    
                    skip_newlines_spanned(iter);
                    
                    match peek_token(iter) {
                        Some(Token::Comma) => {
                            next_spanned(iter);
                        }
                        Some(Token::RightParen) => {}
                        Some(tok) => bail!("Expected ',' or ')' in argument list, got {:?}", tok),
                        None => bail!("Expected ')' to close argument list"),
                    }
                }
                Some(Token::Identifier(_)) => {
                    if let Some((Token::Identifier(id), span)) = next_spanned(iter) {
                        args.push(Argument::Identifier(id, span));
                    }
                    
                    skip_newlines_spanned(iter);
                    
                    match peek_token(iter) {
                        Some(Token::Comma) => {
                            next_spanned(iter);
                        }
                        Some(Token::RightParen) => {}
                        Some(tok) => bail!("Expected ',' or ')' in argument list, got {:?}", tok),
                        None => bail!("Expected ')' to close argument list"),
                    }
                }
                Some(Token::LeftBracket) => {
                    let array_start_span = next_spanned(iter).unwrap().1;
                    let mut array_items = Vec::new();
                    
                    loop {
                        skip_newlines_spanned(iter);
                        
                        match peek_token(iter) {
                            Some(Token::RightBracket) => {
                                let end_span = next_spanned(iter).unwrap().1;
                                let merged_span = array_start_span.merge(&end_span);
                                args.push(Argument::Array(array_items, merged_span));
                                break;
                            }
                            Some(Token::String(_)) => {
                                if let Some((Token::String(s), span)) = next_spanned(iter) {
                                    array_items.push(Argument::String(s, span));
                                }
                            }
                            Some(Token::Number(_)) => {
                                if let Some((Token::Number(n), span)) = next_spanned(iter) {
                                    array_items.push(Argument::Number(n, span));
                                }
                            }
                            Some(Token::Identifier(_)) => {
                                if let Some((Token::Identifier(id), span)) = next_spanned(iter) {
                                    array_items.push(Argument::Identifier(id, span));
                                }
                            }
                            Some(tok) => bail!("Expected array item or ']', got {:?}", tok),
                            None => bail!("Expected ']' to close array literal"),
                        }
                        
                        skip_newlines_spanned(iter);
                        
                        match peek_token(iter) {
                            Some(Token::Comma) => {
                                next_spanned(iter);
                            }
                            Some(Token::RightBracket) => {
                            }
                            Some(tok) => bail!("Expected ',' or ']' in array, got {:?}", tok),
                            None => bail!("Expected ']' to close array"),
                        }
                    }
                    
                    skip_newlines_spanned(iter);
                    
                    match peek_token(iter) {
                        Some(Token::Comma) => {
                            next_spanned(iter);
                        }
                        Some(Token::RightParen) => {}
                        Some(tok) => bail!("Expected ',' or ')' after array argument, got {:?}", tok),
                        None => bail!("Expected ')' to close argument list"),
                    }
                }
                Some(tok) => bail!("Expected argument or ')' in argument list, got {:?}", tok),
                None => bail!("Expected ')' to close argument list"),
            }
        }

        args
    } else {
        Vec::new()
    };

    Ok(Statement::MacroCall {
        namespace,
        name,
        args,
        span: start_span,
    })
}

fn parse_conditional_rule_spanned(iter: &mut std::iter::Peekable<std::vec::IntoIter<SpannedToken>>, is_block: bool) -> Result<Statement> {
    let (_, start_span) = next_spanned(iter).unwrap();
    skip_newlines_spanned(iter);

    let condition = parse_condition_spanned(iter)?;
    skip_newlines_spanned(iter);

    let message = if matches!(peek_token(iter), Some(Token::Message)) {
        next_spanned(iter);
        skip_newlines_spanned(iter);
        
        match next_spanned(iter) {
            Some((Token::String(s), _)) => Some(s),
            Some((tok, span)) => bail!("Expected string after 'message', got {:?} at {:?}", tok, span),
            None => bail!("Expected string after 'message'"),
        }
    } else {
        None
    };

    skip_newlines_spanned(iter);

    let interactive = if matches!(peek_token(iter), Some(Token::Interactive)) {
        next_spanned(iter);
        skip_newlines_spanned(iter);
        
        match next_spanned(iter) {
            Some((Token::String(s), _)) => Some(s),
            Some((tok, span)) => bail!("Expected string after 'interactive', got {:?} at {:?}", tok, span),
            None => bail!("Expected string after 'interactive'"),
        }
    } else {
        None
    };

    let severity = if is_block {
        RuleSeverity::Block(start_span)
    } else {
        RuleSeverity::Warn(start_span)
    };

    Ok(Statement::ConditionalRule {
        severity,
        condition,
        message,
        interactive,
        span: start_span,
    })
}
