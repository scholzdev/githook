use anyhow::{Result, bail};
use std::collections::HashMap;

// ============================================================================
// VALUE SYSTEM V2 - Object-based unified system
// ============================================================================

#[derive(Debug, Clone)]
pub enum Value {
    // Primitives
    String(String),
    Number(f64),
    Bool(bool),
    Null,
    
    // Composite
    Array(Vec<Value>),
    Object(Object),
}

#[derive(Debug, Clone)]
pub struct Object {
    pub type_name: String,
    pub properties: HashMap<String, Value>,
    pub file_context: Option<githook_git::FileContext>,
    pub path_context: Option<githook_git::PathContext>,
}

impl Object {
    pub fn new(type_name: impl Into<String>) -> Self {
        Self {
            type_name: type_name.into(),
            properties: HashMap::new(),
            file_context: None,
            path_context: None,
        }
    }
    
    pub fn with_property(mut self, key: impl Into<String>, value: Value) -> Self {
        self.properties.insert(key.into(), value);
        self
    }
    
    pub fn with_file_context(mut self, ctx: githook_git::FileContext) -> Self {
        self.file_context = Some(ctx);
        self
    }
    
    pub fn with_path_context(mut self, ctx: githook_git::PathContext) -> Self {
        self.path_context = Some(ctx);
        self
    }
    
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.properties.get(key)
    }
    
    pub fn set(&mut self, key: impl Into<String>, value: Value) {
        self.properties.insert(key.into(), value);
    }
}

impl Value {
    // ========================================================================
    // BUILTIN OBJECTS
    // ========================================================================
    
    /// Creates the `env` object for accessing environment variables
    pub fn env_object() -> Self {
        let mut obj = Object::new("env");
        
        // Add common environment variables
        if let Ok(val) = std::env::var("USER") {
            obj.set("USER", Value::String(val));
        }
        if let Ok(val) = std::env::var("HOME") {
            obj.set("HOME", Value::String(val));
        }
        if let Ok(val) = std::env::var("PATH") {
            obj.set("PATH", Value::String(val));
        }
        if let Ok(val) = std::env::var("PWD") {
            obj.set("PWD", Value::String(val));
        }
        if let Ok(val) = std::env::var("SHELL") {
            obj.set("SHELL", Value::String(val));
        }
        
        Value::Object(obj)
    }
    
    // ========================================================================
    // TYPE CHECKS
    // ========================================================================
    
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }
    
    pub fn is_number(&self) -> bool {
        matches!(self, Value::Number(_))
    }
    
    pub fn is_bool(&self) -> bool {
        matches!(self, Value::Bool(_))
    }
    
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }
    
    pub fn is_array(&self) -> bool {
        matches!(self, Value::Array(_))
    }
    
    pub fn is_object(&self) -> bool {
        matches!(self, Value::Object(_))
    }
    
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Null => false,
            Value::String(s) => !s.is_empty(),
            Value::Number(n) => *n != 0.0,
            Value::Array(arr) => !arr.is_empty(),
            Value::Object(_) => true,
        }
    }
    
    pub fn as_string(&self) -> Result<String> {
        match self {
            Value::String(s) => Ok(s.clone()),
            Value::Number(n) => Ok(n.to_string()),
            Value::Bool(b) => Ok(b.to_string()),
            Value::Null => Ok("null".to_string()),
            _ => bail!("Cannot convert {:?} to string", self),
        }
    }
    
    pub fn as_number(&self) -> Result<f64> {
        match self {
            Value::Number(n) => Ok(*n),
            Value::String(s) => s.parse().map_err(|_| anyhow::anyhow!("Cannot parse '{}' as number", s)),
            _ => bail!("Cannot convert {:?} to number", self),
        }
    }
    
    pub fn as_bool(&self) -> Result<bool> {
        match self {
            Value::Bool(b) => Ok(*b),
            _ => Ok(self.is_truthy()),
        }
    }
    
    // ========================================================================
    // PROPERTY ACCESS
    // ========================================================================
    
    pub fn get_property(&self, name: &str) -> Result<Value> {
        match self {
            Value::Object(obj) => {
                // Try call_property() for File context first
                if obj.type_name == "File" {
                    if let Some(ctx) = &obj.file_context {
                        if let Ok(value) = ctx.call_property(name) {
                            return Ok(value);
                        }
                    }
                }
                // Try call_property() for Path context
                if obj.type_name == "Path" {
                    if let Some(ctx) = &obj.path_context {
                        if let Ok(value) = ctx.call_property(name) {
                            return Ok(value);
                        }
                    }
                }
                // Fall back to map lookup
                obj.get(name)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("Property '{}' not found on {}", name, obj.type_name))
            }
            Value::String(s) => self.get_string_property(s, name),
            Value::Array(arr) => self.get_array_property(arr, name),
            _ => bail!("Cannot access property '{}' on {:?}", name, self),
        }
    }
    
    fn get_string_property(&self, s: &str, name: &str) -> Result<Value> {
        match name {
            "length" => Ok(Value::Number(s.len() as f64)),
            "upper" => Ok(Value::String(s.to_uppercase())),
            "lower" => Ok(Value::String(s.to_lowercase())),
            _ => bail!("Unknown string property: {}", name),
        }
    }
    
    fn get_array_property(&self, arr: &[Value], name: &str) -> Result<Value> {
        match name {
            "length" => Ok(Value::Number(arr.len() as f64)),
            _ => bail!("Unknown array property: {}", name),
        }
    }
    
    // ========================================================================
    // METHOD CALLS
    // ========================================================================
    
    pub fn call_method(&self, name: &str, args: &[Value]) -> Result<Value> {
        match self {
            Value::String(s) => self.call_string_method(s, name, args),
            Value::Number(n) => self.call_number_method(*n, name, args),
            Value::Array(arr) => self.call_array_method(arr, name, args),
            Value::Object(obj) => self.call_object_method(obj, name, args),
            _ => bail!("Cannot call method '{}' on {:?}", name, self),
        }
    }
    
    fn call_string_method(&self, s: &str, name: &str, args: &[Value]) -> Result<Value> {
        match name {
            "len" | "length" => {
                if !args.is_empty() {
                    bail!("length() expects no arguments, got {}", args.len());
                }
                Ok(Value::Number(s.len() as f64))
            }
            
            "is_empty" => {
                if !args.is_empty() {
                    bail!("is_empty() expects no arguments, got {}", args.len());
                }
                Ok(Value::Bool(s.is_empty()))
            }
            
            "to_lowercase" => {
                if !args.is_empty() {
                    bail!("to_lowercase() expects no arguments, got {}", args.len());
                }
                Ok(Value::String(s.to_lowercase()))
            }
            
            "to_uppercase" => {
                if !args.is_empty() {
                    bail!("to_uppercase() expects no arguments, got {}", args.len());
                }
                Ok(Value::String(s.to_uppercase()))
            }
            
            "trim" => {
                if !args.is_empty() {
                    bail!("trim() expects no arguments, got {}", args.len());
                }
                Ok(Value::String(s.trim().to_string()))
            }
            
            "replace" => {
                if args.len() != 2 {
                    bail!("replace() expects 2 arguments (from, to), got {}", args.len());
                }
                let from = args[0].as_string()?;
                let to = args[1].as_string()?;
                Ok(Value::String(s.replace(&from, &to)))
            }
            
            "contains" => {
                if args.len() != 1 {
                    bail!("contains() expects 1 argument, got {}", args.len());
                }
                let needle = args[0].as_string()?;
                Ok(Value::Bool(s.contains(&needle)))
            }
            
            "starts_with" => {
                if args.len() != 1 {
                    bail!("starts_with() expects 1 argument, got {}", args.len());
                }
                let prefix = args[0].as_string()?;
                Ok(Value::Bool(s.starts_with(&prefix)))
            }
            
            "ends_with" => {
                if args.len() != 1 {
                    bail!("ends_with() expects 1 argument, got {}", args.len());
                }
                let suffix = args[0].as_string()?;
                Ok(Value::Bool(s.ends_with(&suffix)))
            }
            
            "matches" => {
                if args.len() != 1 {
                    bail!("matches() expects 1 argument, got {}", args.len());
                }
                let pattern = args[0].as_string()?;
                let regex = regex::Regex::new(&pattern)?;
                Ok(Value::Bool(regex.is_match(s)))
            }
            
            "split" => {
                if args.len() != 1 {
                    bail!("split() expects 1 argument, got {}", args.len());
                }
                let delimiter = args[0].as_string()?;
                let parts: Vec<Value> = s.split(&delimiter)
                    .map(|p| Value::String(p.to_string()))
                    .collect();
                Ok(Value::Array(parts))
            }
            
            _ => bail!("Unknown string method: {}", name),
        }
    }
    
    fn call_number_method(&self, n: f64, name: &str, args: &[Value]) -> Result<Value> {
        match name {
            "abs" => {
                if !args.is_empty() {
                    bail!("abs() expects no arguments, got {}", args.len());
                }
                Ok(Value::Number(n.abs()))
            }
            
            "floor" => {
                if !args.is_empty() {
                    bail!("floor() expects no arguments, got {}", args.len());
                }
                Ok(Value::Number(n.floor()))
            }
            
            "ceil" => {
                if !args.is_empty() {
                    bail!("ceil() expects no arguments, got {}", args.len());
                }
                Ok(Value::Number(n.ceil()))
            }
            
            "round" => {
                if !args.is_empty() {
                    bail!("round() expects no arguments, got {}", args.len());
                }
                Ok(Value::Number(n.round()))
            }
            
            _ => bail!("Unknown number method: {}", name),
        }
    }
    
    fn call_array_method(&self, arr: &[Value], name: &str, _args: &[Value]) -> Result<Value> {
        match name {
            "first" => {
                arr.first()
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("Array is empty"))
            }
            
            "last" => {
                arr.last()
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("Array is empty"))
            }
            
            "len" | "length" => {
                Ok(Value::Number(arr.len() as f64))
            }
            
            "join" => {
                let separator = _args.first()
                    .map(|v| v.as_string())
                    .transpose()?
                    .unwrap_or_else(|| ", ".to_string());
                
                let strings: Vec<String> = arr.iter()
                    .map(|v| v.display())
                    .collect();
                
                Ok(Value::String(strings.join(&separator)))
            }
            
            "reverse" => {
                let mut reversed = arr.to_vec();
                reversed.reverse();
                Ok(Value::Array(reversed))
            }
            
            "sort" => {
                let mut sorted = arr.to_vec();
                sorted.sort_by(|a, b| {
                    match (a, b) {
                        (Value::Number(x), Value::Number(y)) => {
                            x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal)
                        }
                        (Value::String(x), Value::String(y)) => x.cmp(y),
                        _ => std::cmp::Ordering::Equal,
                    }
                });
                Ok(Value::Array(sorted))
            }
            
            "filter" => {
                // Handled by executor with closures
                bail!("filter() requires a closure argument");
            }
            
            "map" => {
                // Handled by executor with closures
                bail!("map() requires a closure argument");
            }
            
            "find" => {
                // Handled by executor with closures
                bail!("find() requires a closure argument");
            }
            
            "any" => {
                // Handled by executor with closures
                bail!("any() requires a closure argument");
            }
            
            "all" => {
                // Handled by executor with closures
                bail!("all() requires a closure argument");
            }
            
            _ => bail!("Unknown array method: {}", name),
        }
    }
    
    fn call_object_method(&self, obj: &Object, name: &str, args: &[Value]) -> Result<Value> {
        // File object methods
        if obj.type_name == "File" {
            if let Some(ctx) = &obj.file_context {
                // Convert Value args to strings for the call
                let string_args: Result<Vec<String>> = args.iter()
                    .map(|v| v.as_string())
                    .collect();
                let string_args = string_args?;
                let str_refs: Vec<&str> = string_args.iter().map(|s| s.as_str()).collect();
                ctx.call_method(name, &str_refs)
            } else {
                bail!("File object has no context")
            }
        } 
        // Path object methods
        else if obj.type_name == "Path" {
            if let Some(ctx) = &obj.path_context {
                // Convert Value args to strings for the call
                let string_args: Result<Vec<String>> = args.iter()
                    .map(|v| v.as_string())
                    .collect();
                let string_args = string_args?;
                let str_refs: Vec<&str> = string_args.iter().map(|s| s.as_str()).collect();
                ctx.call_method(name, &str_refs)
            } else {
                bail!("Path object has no context")
            }
        } 
        else {
            bail!("Method '{}' not found on {}", name, obj.type_name)
        }
    }
    
    // ========================================================================
    // DISPLAY
    // ========================================================================
    
    pub fn display(&self) -> String {
        match self {
            Value::String(s) => s.clone(),
            Value::Number(n) => {
                if n.fract() == 0.0 {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            Value::Bool(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.display()).collect();
                format!("[{}]", items.join(", "))
            }
            Value::Object(obj) => {
                // Special handling for PathContext - use Display trait
                if let Some(path_ctx) = &obj.path_context {
                    return path_ctx.to_string();
                }
                // Default object display
                format!("{}{{ {} properties }}", obj.type_name, obj.properties.len())
            }
        }
    }
}

// ============================================================================
// COMPARISON OPERATIONS
// ============================================================================

impl Value {
    pub fn equals(&self, other: &Value) -> Result<bool> {
        match (self, other) {
            (Value::String(a), Value::String(b)) => Ok(a == b),
            (Value::Number(a), Value::Number(b)) => Ok((a - b).abs() < f64::EPSILON),
            (Value::Bool(a), Value::Bool(b)) => Ok(a == b),
            (Value::Null, Value::Null) => Ok(true),
            _ => Ok(false),
        }
    }
    
    pub fn not_equals(&self, other: &Value) -> Result<bool> {
        Ok(!self.equals(other)?)
    }
    
    pub fn less_than(&self, other: &Value) -> Result<bool> {
        let a = self.as_number()?;
        let b = other.as_number()?;
        Ok(a < b)
    }
    
    pub fn less_or_equal(&self, other: &Value) -> Result<bool> {
        let a = self.as_number()?;
        let b = other.as_number()?;
        Ok(a <= b)
    }
    
    pub fn greater_than(&self, other: &Value) -> Result<bool> {
        let a = self.as_number()?;
        let b = other.as_number()?;
        Ok(a > b)
    }
    
    pub fn greater_or_equal(&self, other: &Value) -> Result<bool> {
        let a = self.as_number()?;
        let b = other.as_number()?;
        Ok(a >= b)
    }
}

// ============================================================================
// CONSTRUCTORS FOR COMMON OBJECTS
// ============================================================================

impl Value {
    pub fn file_object(path: String) -> Value {
        let file_ctx = githook_git::FileContext::from_path(&path);
        
        // Create Path object from FileContext's path
        let path_obj = Value::Object(
            Object::new("Path")
                .with_path_context(file_ctx.path.clone())
        );
        
        Value::Object(
            Object::new("File")
                .with_property("path", path_obj)
                .with_file_context(file_ctx)
        )
    }
    
    pub fn path_object(path: String) -> Value {
        let path_ctx = githook_git::PathContext::from_path(&path);
        Value::Object(
            Object::new("Path")
                .with_path_context(path_ctx)
        )
    }
}

// ============================================================================
// FROM IMPLEMENTATIONS
// ============================================================================

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<f64> for Value {
    fn from(n: f64) -> Self {
        Value::Number(n)
    }
}

