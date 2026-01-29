use githook_syntax::lexer::{tokenize, Token};

#[test]
fn test_keywords() {
    let source = "run block warn allow parallel let foreach if else match where macro import use group";
    let tokens = tokenize(source).unwrap();
    
    let keywords: Vec<_> = tokens.iter()
        .map(|t| &t.token)
        .collect();
    
    assert!(matches!(keywords[0], Token::Run));
    assert!(matches!(keywords[1], Token::Block));
    assert!(matches!(keywords[2], Token::Warn));
    assert!(matches!(keywords[3], Token::Allow));
    assert!(matches!(keywords[4], Token::Parallel));
    assert!(matches!(keywords[5], Token::Let));
    assert!(matches!(keywords[6], Token::Foreach));
    assert!(matches!(keywords[7], Token::If));
    assert!(matches!(keywords[8], Token::Else));
    assert!(matches!(keywords[9], Token::Match));
    assert!(matches!(keywords[10], Token::Where));
    assert!(matches!(keywords[11], Token::Macro));
    assert!(matches!(keywords[12], Token::Import));
    assert!(matches!(keywords[13], Token::Use));
    assert!(matches!(keywords[14], Token::Group));
}

#[test]
fn test_operators() {
    let source = "== != < <= > >= = and or not";
    let tokens = tokenize(source).unwrap();
    
    let ops: Vec<_> = tokens.iter().map(|t| &t.token).collect();
    
    assert!(matches!(ops[0], Token::Eq));
    assert!(matches!(ops[1], Token::Ne));
    assert!(matches!(ops[2], Token::Lt));
    assert!(matches!(ops[3], Token::Le));
    assert!(matches!(ops[4], Token::Gt));
    assert!(matches!(ops[5], Token::Ge));
    assert!(matches!(ops[6], Token::Assign));
    assert!(matches!(ops[7], Token::And));
    assert!(matches!(ops[8], Token::Or));
    assert!(matches!(ops[9], Token::Not));
}

#[test]
fn test_strings() {
    let source = r#""hello" "world with spaces" "escaped \"quotes\"" "newline\n""#;
    let tokens = tokenize(source).unwrap();
    
    match &tokens[0].token {
        Token::String(s) => assert_eq!(s, "hello"),
        _ => panic!("Expected string token"),
    }
    
    match &tokens[1].token {
        Token::String(s) => assert_eq!(s, "world with spaces"),
        _ => panic!("Expected string token"),
    }
    
    match &tokens[2].token {
        Token::String(s) => assert_eq!(s, "escaped \"quotes\""),
        _ => panic!("Expected string token"),
    }
    
    match &tokens[3].token {
        Token::String(s) => assert_eq!(s, "newline\n"),
        _ => panic!("Expected string token"),
    }
}

#[test]
fn test_numbers() {
    let source = "42 3.14 0.5 1000000";
    let tokens = tokenize(source).unwrap();
    
    match &tokens[0].token {
        Token::Number(n) => assert_eq!(*n, 42.0),
        _ => panic!("Expected number"),
    }
    
    match &tokens[1].token {
        Token::Number(n) => assert_eq!(*n, 3.14),
        _ => panic!("Expected number"),
    }
    
    match &tokens[2].token {
        Token::Number(n) => assert_eq!(*n, 0.5),
        _ => panic!("Expected number"),
    }
    
    match &tokens[3].token {
        Token::Number(n) => assert_eq!(*n, 1000000.0),
        _ => panic!("Expected number"),
    }
}

#[test]
fn test_identifiers() {
    let source = "file git branch_name camelCase snake_case";
    let tokens = tokenize(source).unwrap();
    
    match &tokens[0].token {
        Token::Identifier(id) => assert_eq!(id, "file"),
        _ => panic!("Expected identifier"),
    }
    
    match &tokens[1].token {
        Token::Identifier(id) => assert_eq!(id, "git"),
        _ => panic!("Expected identifier"),
    }
}

#[test]
fn test_comments() {
    let source = r#"
run "test" # This is a comment
# Another comment
block if true # inline comment
"#;
    let tokens = tokenize(source).unwrap();
    
    // Comments should be skipped, only run, string, block, if, identifier tokens
    assert!(tokens.iter().all(|t| !matches!(t.token, Token::Identifier(ref id) if id.starts_with('#'))));
}

#[test]
fn test_property_access() {
    let source = "file.size git.all_files branch.name";
    let tokens = tokenize(source).unwrap();
    
    // file . size
    assert!(matches!(tokens[0].token, Token::Identifier(_)));
    assert!(matches!(tokens[1].token, Token::Dot));
    assert!(matches!(tokens[2].token, Token::Identifier(_)));
}

#[test]
fn test_blocks() {
    let source = "{ } [ ] ( )";
    let tokens = tokenize(source).unwrap();
    
    assert!(matches!(tokens[0].token, Token::LeftBrace));
    assert!(matches!(tokens[1].token, Token::RightBrace));
    assert!(matches!(tokens[2].token, Token::LeftBracket));
    assert!(matches!(tokens[3].token, Token::RightBracket));
    assert!(matches!(tokens[4].token, Token::LeftParen));
    assert!(matches!(tokens[5].token, Token::RightParen));
}

#[test]
fn test_real_world_snippet() {
    let source = r#"
foreach git.all_files { file in
    block if file.size > 1000000 message "Too large"
    warn if file.extension == "rs" and file.size > 50000 message "Large Rust file"
}
"#;
    let tokens = tokenize(source).unwrap();
    
    // Should have: newlines, foreach, git, ., all_files, {, file, in, ...
    assert!(tokens.len() > 20);
    
    // Find the foreach token (skip newlines)
    let foreach_pos = tokens.iter().position(|t| matches!(t.token, Token::Foreach));
    assert!(foreach_pos.is_some(), "Should contain Foreach token");
}

#[test]
fn test_string_interpolation_tokens() {
    let source = r#"run "echo ${file.path}""#;
    let tokens = tokenize(source).unwrap();
    
    // Lexer doesn't parse interpolation, that's parser's job
    // String should contain ${file.path} literally
    match &tokens[1].token {
        Token::String(s) => assert!(s.contains("${file.path}")),
        _ => panic!("Expected string"),
    }
}

#[test]
fn test_multiline() {
    let source = "run\n\"test\"\nblock\nif\ntrue";
    let tokens = tokenize(source).unwrap();
    
    // Check that newlines are present
    let has_newlines = tokens.iter().any(|t| matches!(t.token, Token::Newline));
    assert!(has_newlines);
}
