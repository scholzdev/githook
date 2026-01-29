use githook_eval::executor::Executor;
use githook_syntax::{lexer, parser};

#[test]
fn test_env_variables() {
    let code = r#"
let user = env.USER
"#;
    
    let tokens = lexer::tokenize(code).unwrap();
    let statements = parser::parse(&tokens).unwrap();
    
    let executor = Executor::new();
    for stmt in &statements {
        executor.execute(stmt).unwrap();
    }
    
    // Check that USER was assigned
    let user_val = executor.variables.get("user").unwrap();
    assert!(matches!(user_val, githook_eval::value::Value::String(_)));
}

#[test]
fn test_env_properties() {
    let code = r#"
let home = env.HOME
let path = env.PATH
"#;
    
    let tokens = lexer::tokenize(code).unwrap();
    let statements = parser::parse(&tokens).unwrap();
    
    let executor = Executor::new();
    for stmt in &statements {
        executor.execute(stmt).unwrap();
    }
    
    // Check that both were assigned
    assert!(executor.variables.contains_key("home"));
    assert!(executor.variables.contains_key("path"));
}
