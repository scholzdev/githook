use githook_eval::{Executor, ExecutionResult};
use githook_eval::value::{Value, Object};
use githook_syntax::{lexer, parser};
use anyhow::Result;

#[test]
fn test_simple_run() -> Result<()> {
    let source = r#"run "echo test""#;
    let tokens = lexer::tokenize(source)?;
    let statements = parser::parse(tokens)?;
    
    let mut executor = Executor::new();
    let result = executor.execute_statements(&statements)?;
    
    assert_eq!(result, ExecutionResult::Continue);
    Ok(())
}

#[test]
fn test_let_and_variable() -> Result<()> {
    let source = r#"
let x = 42
let y = "hello"
"#;
    let tokens = lexer::tokenize(source)?;
    let statements = parser::parse(tokens)?;
    
    let mut executor = Executor::new();
    executor.execute_statements(&statements)?;
    
    // Check variables were set
    match executor.variables.get("x") {
        Some(Value::Number(n)) => assert_eq!(*n, 42.0),
        _ => panic!("Expected x = 42"),
    }
    
    match executor.variables.get("y") {
        Some(Value::String(s)) => assert_eq!(s, "hello"),
        _ => panic!("Expected y = hello"),
    }
    
    Ok(())
}

#[test]
fn test_if_true() -> Result<()> {
    let source = r#"
let x = 0
if 1 {
    let x = 1
}
"#;
    let tokens = lexer::tokenize(source)?;
    let statements = parser::parse(tokens)?;
    
    let mut executor = Executor::new();
    let result = executor.execute_statements(&statements)?;
    
    assert_eq!(result, ExecutionResult::Continue);
    Ok(())
}

#[test]
fn test_if_false_with_else() -> Result<()> {
    let source = r#"
let result = "none"
if 0 {
    let result = "then"
} else {
    let result = "else"
}
"#;
    let tokens = lexer::tokenize(source)?;
    let statements = parser::parse(tokens)?;
    
    let mut executor = Executor::new();
    executor.execute_statements(&statements)?;
    
    // After the if/else, result should be "else"
    match executor.variables.get("result") {
        Some(Value::String(s)) => assert_eq!(s, "else"),
        _ => panic!("Expected result = else"),
    }
    
    Ok(())
}

#[test]
fn test_block_if_blocks() -> Result<()> {
    let source = r#"block if 1 message "Blocked""#;
    let tokens = lexer::tokenize(source)?;
    let statements = parser::parse(tokens)?;
    
    let mut executor = Executor::new();
    let result = executor.execute_statements(&statements)?;
    
    assert_eq!(result, ExecutionResult::Blocked);
    Ok(())
}

#[test]
fn test_block_if_continues() -> Result<()> {
    let source = r#"block if 0 message "Should not block""#;
    let tokens = lexer::tokenize(source)?;
    let statements = parser::parse(tokens)?;
    
    let mut executor = Executor::new();
    let result = executor.execute_statements(&statements)?;
    
    assert_eq!(result, ExecutionResult::Continue);
    Ok(())
}

#[test]
fn test_foreach_array() -> Result<()> {
    let source = r#"
let items = ["a", "b", "c"]
foreach items { item in
    run "echo test"
}
"#;
    let tokens = lexer::tokenize(source)?;
    let statements = parser::parse(tokens)?;
    
    let mut executor = Executor::new();
    let result = executor.execute_statements(&statements)?;
    
    assert_eq!(result, ExecutionResult::Continue);
    Ok(())
}

#[test]
fn test_foreach_git_files() -> Result<()> {
    let source = r#"
foreach git.all_files { file in
    run "echo test"
}
"#;
    let tokens = lexer::tokenize(source)?;
    let statements = parser::parse(tokens)?;
    
    let files = vec!["test1.txt".to_string(), "test2.txt".to_string()];
    let mut executor = Executor::new().with_git_files(files);
    
    let result = executor.execute_statements(&statements)?;
    
    assert_eq!(result, ExecutionResult::Continue);
    Ok(())
}

#[test]
fn test_comparison_operators() -> Result<()> {
    let source = r#"
let x = 10
let y = 20
block if x > y message "Should not block"
block if y > x message "Should block"
"#;
    let tokens = lexer::tokenize(source)?;
    let statements = parser::parse(tokens)?;
    
    let mut executor = Executor::new();
    let result = executor.execute_statements(&statements)?;
    
    // Second block if should trigger
    assert_eq!(result, ExecutionResult::Blocked);
    Ok(())
}

#[test]
fn test_logical_and() -> Result<()> {
    let source = r#"
let x = 10
let y = 20
block if x > 5 and y > 15 message "Both true"
"#;
    let tokens = lexer::tokenize(source)?;
    let statements = parser::parse(tokens)?;
    
    let mut executor = Executor::new();
    let result = executor.execute_statements(&statements)?;
    
    assert_eq!(result, ExecutionResult::Blocked);
    Ok(())
}

#[test]
fn test_logical_or() -> Result<()> {
    let source = r#"
let x = 3
let y = 20
block if x > 5 or y > 15 message "One true"
"#;
    let tokens = lexer::tokenize(source)?;
    let statements = parser::parse(tokens)?;
    
    let mut executor = Executor::new();
    let result = executor.execute_statements(&statements)?;
    
    assert_eq!(result, ExecutionResult::Blocked);
    Ok(())
}

#[test]
fn test_string_interpolation() -> Result<()> {
    let source = r#"
let name = "World"
run "echo Hello ${name}!"
"#;
    let tokens = lexer::tokenize(source)?;
    let statements = parser::parse(tokens)?;
    
    let mut executor = Executor::new();
    let result = executor.execute_statements(&statements)?;
    
    // Should execute without error, interpolation happens in run command
    assert_eq!(result, ExecutionResult::Continue);
    
    Ok(())
}

#[test]
fn test_string_interpolation_property() -> Result<()> {
    let source = r#"
foreach git.all_files { file in
    run "echo File: ${file.path}"
}
"#;
    let tokens = lexer::tokenize(source)?;
    let statements = parser::parse(tokens)?;
    
    let files = vec!["test.txt".to_string()];
    let mut executor = Executor::new().with_git_files(files);
    
    let result = executor.execute_statements(&statements)?;
    
    // Should execute and interpolate file.path
    assert_eq!(result, ExecutionResult::Continue);
    
    Ok(())
}

#[test]
fn test_match_literal() -> Result<()> {
    let source = r#"
let ext = "rs"
match ext {
    "rs" -> run "echo Rust"
    "py" -> run "echo Python"
    _ -> run "echo Other"
}
"#;
    let tokens = lexer::tokenize(source)?;
    let statements = parser::parse(tokens)?;
    
    let mut executor = Executor::new();
    let result = executor.execute_statements(&statements)?;
    
    assert_eq!(result, ExecutionResult::Continue);
    Ok(())
}

#[test]
fn test_match_wildcard() -> Result<()> {
    let source = r#"
let file = "test.rs"
match file {
    "*.rs" -> run "echo Rust file"
    "*.py" -> run "echo Python file"
    _ -> run "echo Other"
}
"#;
    let tokens = lexer::tokenize(source)?;
    let statements = parser::parse(tokens)?;
    
    let mut executor = Executor::new();
    let result = executor.execute_statements(&statements)?;
    
    assert_eq!(result, ExecutionResult::Continue);
    Ok(())
}

#[test]
fn test_property_access_chain() -> Result<()> {
    let source = r#"
block if repo.branch.name == "main" message "On main branch"
"#;
    let tokens = lexer::tokenize(source)?;
    let statements = parser::parse(tokens)?;
    
    let mut executor = Executor::new();
    
    // Create nested structure: repo.branch.name
    let mut branch = Object::new("Branch");
    branch.set("name", Value::String("main".to_string()));
    
    let mut repo = Object::new("Repo");
    repo.set("branch", Value::Object(branch));
    
    executor.variables.insert("repo".to_string(), Value::Object(repo));
    
    // Execute - should block because repo.branch.name == "main" is true
    let result = executor.execute_statements(&statements)?;
    
    assert_eq!(result, ExecutionResult::Blocked);
    
    Ok(())
}

#[test]
fn test_group_execution() -> Result<()> {
    let source = r#"
group test {
    run "echo test1"
    run "echo test2"
}
"#;
    let tokens = lexer::tokenize(source)?;
    let statements = parser::parse(tokens)?;
    
    let mut executor = Executor::new();
    let result = executor.execute_statements(&statements)?;
    
    assert_eq!(result, ExecutionResult::Continue);
    Ok(())
}

#[test]
#[ignore] // TODO: Parser doesn't support 'disabled' keyword yet, only 'enabled'
fn test_disabled_group() -> Result<()> {
    let source = r#"
group test disabled {
    block if true message "Should not execute"
}
"#;
    let tokens = lexer::tokenize(source)?;
    let statements = parser::parse(tokens)?;
    
    let mut executor = Executor::new();
    let result = executor.execute_statements(&statements)?;
    
    // Group is disabled, so block should not execute
    assert_eq!(result, ExecutionResult::Continue);
    Ok(())
}

#[test]
fn test_not_operator() -> Result<()> {
    let source = r#"
let flag = 0
block if not flag message "Should block"
"#;
    let tokens = lexer::tokenize(source)?;
    let statements = parser::parse(tokens)?;
    
    let mut executor = Executor::new();
    let result = executor.execute_statements(&statements)?;
    
    assert_eq!(result, ExecutionResult::Blocked);
    Ok(())
}
