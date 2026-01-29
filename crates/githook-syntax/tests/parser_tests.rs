use githook_syntax::{lexer, parser};
use githook_syntax::ast::{Statement, Expression, BinaryOp};

#[test]
fn test_simple_run() {
    let source = r#"run "echo hello""#;
    let tokens = lexer::tokenize(source).unwrap();
    let statements = parser::parse(tokens).unwrap();
    
    assert_eq!(statements.len(), 1);
    match &statements[0] {
        Statement::Run { command, .. } => {
            assert_eq!(command, "echo hello");
        }
        _ => panic!("Expected Run statement"),
    }
}

#[test]
fn test_block_if() {
    let source = r#"block if file.size > 1000000 message "Too large""#;
    let tokens = lexer::tokenize(source).unwrap();
    let statements = parser::parse(tokens).unwrap();
    
    assert_eq!(statements.len(), 1);
    match &statements[0] {
        Statement::BlockIf { condition, message, .. } => {
            assert!(matches!(condition, Expression::Binary { .. }));
            assert_eq!(message.as_ref().unwrap(), "Too large");
        }
        _ => panic!("Expected BlockIf statement"),
    }
}

#[test]
fn test_warn_if() {
    let source = r#"warn if x > 10 message "Warning""#;
    let tokens = lexer::tokenize(source).unwrap();
    let statements = parser::parse(tokens).unwrap();
    
    assert_eq!(statements.len(), 1);
    match &statements[0] {
        Statement::WarnIf { message, .. } => {
            assert_eq!(message.as_ref().unwrap(), "Warning");
        }
        _ => panic!("Expected WarnIf statement"),
    }
}

#[test]
fn test_let_statement() {
    let source = r#"let max_size = 1000"#;
    let tokens = lexer::tokenize(source).unwrap();
    let statements = parser::parse(tokens).unwrap();
    
    assert_eq!(statements.len(), 1);
    match &statements[0] {
        Statement::Let { name, .. } => {
            assert_eq!(name, "max_size");
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn test_let_array() {
    let source = r#"let forbidden = ["exe", "dll", "so"]"#;
    let tokens = lexer::tokenize(source).unwrap();
    let statements = parser::parse(tokens).unwrap();
    
    assert_eq!(statements.len(), 1);
    match &statements[0] {
        Statement::Let { name, .. } => {
            assert_eq!(name, "forbidden");
        }
        _ => panic!("Expected Let statement"),
    }
}

#[test]
fn test_foreach() {
    let source = r#"
foreach git.all_files { file in
    run "echo test"
}
"#;
    let tokens = lexer::tokenize(source).unwrap();
    let statements = parser::parse(tokens).unwrap();
    
    assert_eq!(statements.len(), 1);
    match &statements[0] {
        Statement::ForEach { var, body, .. } => {
            assert_eq!(var, "file");
            assert_eq!(body.len(), 1);
        }
        _ => panic!("Expected ForEach statement"),
    }
}

#[test]
fn test_if_else() {
    let source = r#"
if x > 10 {
    warn "Large"
} else {
    run "echo small"
}
"#;
    let tokens = lexer::tokenize(source).unwrap();
    let statements = parser::parse(tokens).unwrap();
    
    assert_eq!(statements.len(), 1);
    match &statements[0] {
        Statement::If { then_body, else_body, .. } => {
            assert_eq!(then_body.len(), 1);
            assert!(else_body.is_some());
            assert_eq!(else_body.as_ref().unwrap().len(), 1);
        }
        _ => panic!("Expected If statement"),
    }
}

#[test]
fn test_match_expression() {
    let source = r#"
match file.extension {
    "rs" -> run "cargo check"
    "py" -> run "python check"
    _ -> allow "any"
}
"#;
    let tokens = lexer::tokenize(source).unwrap();
    let statements = parser::parse(tokens).unwrap();
    
    assert_eq!(statements.len(), 1);
    match &statements[0] {
        Statement::Match { arms, .. } => {
            assert_eq!(arms.len(), 3);
        }
        _ => panic!("Expected Match statement"),
    }
}

#[test]
fn test_property_chain() {
    let source = r#"block if file.size > 100 message "test""#;
    let tokens = lexer::tokenize(source).unwrap();
    let statements = parser::parse(tokens).unwrap();
    
    match &statements[0] {
        Statement::BlockIf { condition, .. } => {
            match condition {
                Expression::Binary { left, op, .. } => {
                    match &**left {
                        Expression::PropertyAccess { chain, .. } => {
                            assert_eq!(chain, &vec!["file".to_string(), "size".to_string()]);
                        }
                        _ => panic!("Expected PropertyAccess"),
                    }
                    assert_eq!(*op, BinaryOp::Gt);
                }
                _ => panic!("Expected Binary expression"),
            }
        }
        _ => panic!("Expected BlockIf"),
    }
}

#[test]
fn test_method_call() {
    let source = r#"block if file.content.contains("TODO") message "Found TODO""#;
    let tokens = lexer::tokenize(source).unwrap();
    let statements = parser::parse(tokens).unwrap();
    
    assert_eq!(statements.len(), 1);
    // Should parse without error
}

#[test]
fn test_logical_operators() {
    let source = r#"block if x > 10 and y < 5 or z == 3 message "test""#;
    let tokens = lexer::tokenize(source).unwrap();
    let statements = parser::parse(tokens).unwrap();
    
    assert_eq!(statements.len(), 1);
    match &statements[0] {
        Statement::BlockIf { condition, .. } => {
            // Should be: ((x > 10) and (y < 5)) or (z == 3)
            match condition {
                Expression::Binary { op: BinaryOp::Or, .. } => {
                    // Correct precedence
                }
                _ => panic!("Expected Or at top level"),
            }
        }
        _ => panic!("Expected BlockIf"),
    }
}

#[test]
fn test_nested_blocks() {
    let source = r#"
if x > 5 {
    if y > 10 {
        warn "Both large"
    }
}
"#;
    let tokens = lexer::tokenize(source).unwrap();
    let statements = parser::parse(tokens).unwrap();
    
    assert_eq!(statements.len(), 1);
    match &statements[0] {
        Statement::If { then_body, .. } => {
            assert_eq!(then_body.len(), 1);
            match &then_body[0] {
                Statement::If { .. } => {
                    // Nested if found
                }
                _ => panic!("Expected nested If"),
            }
        }
        _ => panic!("Expected If statement"),
    }
}

#[test]
fn test_multiple_statements() {
    let source = r#"
run "echo start"
let x = 10
warn "test"
run "echo end"
"#;
    let tokens = lexer::tokenize(source).unwrap();
    let statements = parser::parse(tokens).unwrap();
    
    assert_eq!(statements.len(), 4);
}

#[test]
fn test_group_statement() {
    let source = r#"
group validation {
    run "test1"
    run "test2"
}
"#;
    let tokens = lexer::tokenize(source).unwrap();
    let statements = parser::parse(tokens).unwrap();
    
    assert_eq!(statements.len(), 1);
    match &statements[0] {
        Statement::Group { name, body, .. } => {
            assert_eq!(name, "validation");
            assert_eq!(body.len(), 2);
        }
        _ => panic!("Expected Group statement"),
    }
}

#[test]
fn test_parallel() {
    let source = r#"
parallel {
    run "cargo test"
    run "cargo clippy"
    run "cargo fmt --check"
}
"#;
    let tokens = lexer::tokenize(source).unwrap();
    let statements = parser::parse(tokens).unwrap();
    
    assert_eq!(statements.len(), 1);
    match &statements[0] {
        Statement::Parallel { commands, .. } => {
            assert_eq!(commands.len(), 3);
            assert_eq!(commands[0], "cargo test");
            assert_eq!(commands[1], "cargo clippy");
            assert_eq!(commands[2], "cargo fmt --check");
        }
        _ => panic!("Expected Parallel statement"),
    }
}

#[test]
fn test_string_with_interpolation() {
    let source = r#"run "echo ${file.path}""#;
    let tokens = lexer::tokenize(source).unwrap();
    let statements = parser::parse(tokens).unwrap();
    
    assert_eq!(statements.len(), 1);
    match &statements[0] {
        Statement::Run { command, .. } => {
            assert!(command.contains("${file.path}"));
        }
        _ => panic!("Expected Run statement"),
    }
}
