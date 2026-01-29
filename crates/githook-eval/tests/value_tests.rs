use githook_eval::value::{Value, Object};
use anyhow::Result;

#[test]
fn test_value_truthy() {
    assert!(Value::Bool(true).is_truthy());
    assert!(!Value::Bool(false).is_truthy());
    assert!(Value::String("hello".to_string()).is_truthy());
    assert!(!Value::String("".to_string()).is_truthy());
    assert!(Value::Number(1.0).is_truthy());
    assert!(!Value::Number(0.0).is_truthy());
    assert!(!Value::Null.is_truthy());
}

#[test]
fn test_value_as_string() -> Result<()> {
    assert_eq!(Value::String("hello".to_string()).as_string()?, "hello");
    assert_eq!(Value::Number(42.0).as_string()?, "42");
    assert_eq!(Value::Bool(true).as_string()?, "true");
    assert_eq!(Value::Null.as_string()?, "null");
    Ok(())
}

#[test]
fn test_value_as_number() -> Result<()> {
    assert_eq!(Value::Number(42.5).as_number()?, 42.5);
    assert_eq!(Value::String("123".to_string()).as_number()?, 123.0);
    Ok(())
}

#[test]
fn test_value_comparisons() -> Result<()> {
    let v1 = Value::Number(10.0);
    let v2 = Value::Number(20.0);
    
    assert!(!v1.equals(&v2)?);
    assert!(v1.not_equals(&v2)?);
    assert!(v1.less_than(&v2)?);
    assert!(v1.less_or_equal(&v2)?);
    assert!(v2.greater_than(&v1)?);
    assert!(v2.greater_or_equal(&v1)?);
    
    Ok(())
}

#[test]
fn test_string_property_access() -> Result<()> {
    let s = Value::String("Hello".to_string());
    
    match s.get_property("length")? {
        Value::Number(n) => assert_eq!(n, 5.0),
        _ => panic!("Expected number"),
    }
    
    match s.get_property("upper")? {
        Value::String(upper) => assert_eq!(upper, "HELLO"),
        _ => panic!("Expected string"),
    }
    
    match s.get_property("lower")? {
        Value::String(lower) => assert_eq!(lower, "hello"),
        _ => panic!("Expected string"),
    }
    
    Ok(())
}

#[test]
fn test_string_contains() -> Result<()> {
    let s = Value::String("hello world".to_string());
    let needle = vec![Value::String("world".to_string())];
    
    match s.call_method("contains", &needle)? {
        Value::Bool(b) => assert!(b),
        _ => panic!("Expected bool"),
    }
    
    let needle2 = vec![Value::String("xyz".to_string())];
    match s.call_method("contains", &needle2)? {
        Value::Bool(b) => assert!(!b),
        _ => panic!("Expected bool"),
    }
    
    Ok(())
}

#[test]
fn test_string_starts_with() -> Result<()> {
    let s = Value::String("hello world".to_string());
    let prefix = vec![Value::String("hello".to_string())];
    
    match s.call_method("starts_with", &prefix)? {
        Value::Bool(b) => assert!(b),
        _ => panic!("Expected bool"),
    }
    
    Ok(())
}

#[test]
fn test_string_ends_with() -> Result<()> {
    let s = Value::String("hello world".to_string());
    let suffix = vec![Value::String("world".to_string())];
    
    match s.call_method("ends_with", &suffix)? {
        Value::Bool(b) => assert!(b),
        _ => panic!("Expected bool"),
    }
    
    Ok(())
}

#[test]
fn test_string_split() -> Result<()> {
    let s = Value::String("a,b,c".to_string());
    let delimiter = vec![Value::String(",".to_string())];
    
    match s.call_method("split", &delimiter)? {
        Value::Array(parts) => {
            assert_eq!(parts.len(), 3);
            match &parts[0] {
                Value::String(s) => assert_eq!(s, "a"),
                _ => panic!("Expected string"),
            }
        }
        _ => panic!("Expected array"),
    }
    
    Ok(())
}

#[test]
fn test_array_length() -> Result<()> {
    let arr = Value::Array(vec![
        Value::Number(1.0),
        Value::Number(2.0),
        Value::Number(3.0),
    ]);
    
    match arr.get_property("length")? {
        Value::Number(n) => assert_eq!(n, 3.0),
        _ => panic!("Expected number"),
    }
    
    Ok(())
}

#[test]
fn test_object_property_access() -> Result<()> {
    let mut obj = Object::new("TestObject");
    obj.set("name", Value::String("test".to_string()));
    obj.set("value", Value::Number(42.0));
    
    let val = Value::Object(obj);
    
    match val.get_property("name")? {
        Value::String(s) => assert_eq!(s, "test"),
        _ => panic!("Expected string"),
    }
    
    match val.get_property("value")? {
        Value::Number(n) => assert_eq!(n, 42.0),
        _ => panic!("Expected number"),
    }
    
    Ok(())
}

#[test]
fn test_file_object() {
    let file = Value::file_object("test.rs".to_string());
    
    match file {
        Value::Object(obj) => {
            assert_eq!(obj.type_name, "File");
            
            match obj.get("path").unwrap() {
                Value::String(s) => assert_eq!(s, "test.rs"),
                _ => panic!("Expected path"),
            }
            
            match obj.get("name").unwrap() {
                Value::String(s) => assert_eq!(s, "test.rs"),
                _ => panic!("Expected name"),
            }
            
            match obj.get("extension").unwrap() {
                Value::String(s) => assert_eq!(s, "rs"),
                _ => panic!("Expected extension"),
            }
        }
        _ => panic!("Expected object"),
    }
}

#[test]
fn test_value_display() {
    assert_eq!(Value::String("hello".to_string()).display(), "hello");
    assert_eq!(Value::Number(42.0).display(), "42");
    assert_eq!(Value::Number(42.5).display(), "42.5");
    assert_eq!(Value::Bool(true).display(), "true");
    assert_eq!(Value::Null.display(), "null");
    
    let arr = Value::Array(vec![
        Value::Number(1.0),
        Value::Number(2.0),
    ]);
    assert_eq!(arr.display(), "[1, 2]");
}

#[test]
fn test_string_matches() -> Result<()> {
    let s = Value::String("hello123".to_string());
    let pattern = vec![Value::String(r"hello\d+".to_string())];
    
    match s.call_method("matches", &pattern)? {
        Value::Bool(b) => assert!(b),
        _ => panic!("Expected bool"),
    }
    
    Ok(())
}

#[test]
fn test_nested_property_chains() -> Result<()> {
    // Create nested object: parent.child.value
    let mut child = Object::new("Child");
    child.set("value", Value::Number(100.0));
    
    let mut parent = Object::new("Parent");
    parent.set("child", Value::Object(child));
    
    let val = Value::Object(parent);
    
    // Access parent.child
    let child_obj = val.get_property("child")?;
    
    // Access child.value
    match child_obj.get_property("value")? {
        Value::Number(n) => assert_eq!(n, 100.0),
        _ => panic!("Expected number"),
    }
    
    Ok(())
}
