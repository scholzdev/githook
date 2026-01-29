use githook_eval::{Executor, ExecutionResult};
use githook_syntax::{lexer, parser};
use anyhow::Result;

#[test]
fn test_true_literal() -> Result<()> {
    let source = r#"
let x = true
block if x message "X is true"
"#;
    let tokens = lexer::tokenize(source)?;
    let statements = parser::parse(tokens)?;
    
    let mut executor = Executor::new();
    let result = executor.execute_statements(&statements)?;
    
    // Should block because x is true
    assert_eq!(result, ExecutionResult::Blocked);
    Ok(())
}

#[test]
fn test_false_literal() -> Result<()> {
    let source = r#"
let x = false
block if x message "X is true"
"#;
    let tokens = lexer::tokenize(source)?;
    let statements = parser::parse(tokens)?;
    
    let mut executor = Executor::new();
    let result = executor.execute_statements(&statements)?;
    
    // Should NOT block because x is false
    assert_eq!(result, ExecutionResult::Continue);
    Ok(())
}

#[test]
fn test_boolean_comparison() -> Result<()> {
    let source = r#"
let is_valid = true
let is_ready = false
block if is_valid == true and is_ready == false message "Both conditions met"
"#;
    let tokens = lexer::tokenize(source)?;
    let statements = parser::parse(tokens)?;
    
    let mut executor = Executor::new();
    let result = executor.execute_statements(&statements)?;
    
    // Should block because both conditions are true
    assert_eq!(result, ExecutionResult::Blocked);
    Ok(())
}

#[test]
fn test_boolean_not() -> Result<()> {
    let source = r#"
let enabled = true
block if not enabled message "Disabled"
"#;
    let tokens = lexer::tokenize(source)?;
    let statements = parser::parse(tokens)?;
    
    let mut executor = Executor::new();
    let result = executor.execute_statements(&statements)?;
    
    // Should NOT block because not true = false
    assert_eq!(result, ExecutionResult::Continue);
    Ok(())
}

#[test]
fn test_boolean_in_if() -> Result<()> {
    let source = r#"
let debug_mode = true
if debug_mode {
    block if 1 > 0 message "Debug mode is on"
} else {
    block if 1 > 0 message "Debug mode is off"
}
"#;
    let tokens = lexer::tokenize(source)?;
    let statements = parser::parse(tokens)?;
    
    let mut executor = Executor::new();
    let result = executor.execute_statements(&statements)?;
    
    // Should block with message from then-branch
    assert_eq!(result, ExecutionResult::Blocked);
    Ok(())
}
