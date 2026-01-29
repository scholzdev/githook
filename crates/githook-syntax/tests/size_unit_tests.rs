use githook_syntax::lexer;

#[test]
fn test_size_unit_kb() {
    let code = "5KB";
    let tokens = lexer::tokenize(code).unwrap();
    assert_eq!(tokens.len(), 1);
    if let githook_syntax::lexer::Token::Number(n) = &tokens[0].token {
        assert_eq!(*n, 5.0 * 1024.0);
    } else {
        panic!("Expected Number token");
    }
}

#[test]
fn test_size_unit_mb() {
    let code = "10MB";
    let tokens = lexer::tokenize(code).unwrap();
    assert_eq!(tokens.len(), 1);
    if let githook_syntax::lexer::Token::Number(n) = &tokens[0].token {
        assert_eq!(*n, 10.0 * 1024.0 * 1024.0);
    } else {
        panic!("Expected Number token");
    }
}

#[test]
fn test_size_unit_gb() {
    let code = "2GB";
    let tokens = lexer::tokenize(code).unwrap();
    assert_eq!(tokens.len(), 1);
    if let githook_syntax::lexer::Token::Number(n) = &tokens[0].token {
        assert_eq!(*n, 2.0 * 1024.0 * 1024.0 * 1024.0);
    } else {
        panic!("Expected Number token");
    }
}

#[test]
fn test_size_unit_tb() {
    let code = "1TB";
    let tokens = lexer::tokenize(code).unwrap();
    assert_eq!(tokens.len(), 1);
    if let githook_syntax::lexer::Token::Number(n) = &tokens[0].token {
        assert_eq!(*n, 1.0 * 1024.0 * 1024.0 * 1024.0 * 1024.0);
    } else {
        panic!("Expected Number token");
    }
}

#[test]
fn test_size_unit_lowercase() {
    let code = "5mb";
    let tokens = lexer::tokenize(code).unwrap();
    assert_eq!(tokens.len(), 1);
    if let githook_syntax::lexer::Token::Number(n) = &tokens[0].token {
        assert_eq!(*n, 5.0 * 1024.0 * 1024.0);
    } else {
        panic!("Expected Number token");
    }
}

#[test]
fn test_size_unit_in_comparison() {
    let code = "file.size > 5MB";
    let tokens = lexer::tokenize(code).unwrap();
    // Should have: Identifier, Dot, Identifier, Gt, Number
    assert!(tokens.len() >= 5);
}
