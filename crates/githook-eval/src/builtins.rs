use anyhow::{bail, Context, Result};
use crate::value::Value;
use std::collections::HashMap;

/// Built-in function signature
pub type BuiltinFn = fn(&[Value]) -> Result<Value>;

/// Registry of all built-in functions
#[derive(Clone)]
pub struct BuiltinRegistry {
    functions: HashMap<&'static str, BuiltinFn>,
}

impl BuiltinRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            functions: HashMap::new(),
        };
        
        // Register all built-in functions
        registry.register("file", builtin_file);
        registry.register("dir", builtin_dir);
        registry.register("glob", builtin_glob);
        registry.register("exec", builtin_exec);
        registry.register("http", builtin_http);
        
        registry
    }
    
    fn register(&mut self, name: &'static str, func: BuiltinFn) {
        self.functions.insert(name, func);
    }
    
    pub fn call(&self, name: &str, args: &[Value]) -> Result<Option<Value>> {
        if let Some(func) = self.functions.get(name) {
            Ok(Some(func(args)?))
        } else {
            Ok(None)
        }
    }
    
    pub fn has(&self, name: &str) -> bool {
        self.functions.contains_key(name)
    }
}

impl Default for BuiltinRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Built-in function implementations
// ============================================================================

/// file(path: string) -> File
/// Creates a File object from a path
fn builtin_file(args: &[Value]) -> Result<Value> {
    if args.len() != 1 {
        bail!("file() takes exactly 1 argument, got {}", args.len());
    }
    
    let path = match &args[0] {
        Value::String(s) => s.clone(),
        _ => bail!("file() requires a string path"),
    };
    
    Ok(Value::file_object(path))
}

/// dir(path: string) -> Array<File>
/// Lists all files in a directory
fn builtin_dir(args: &[Value]) -> Result<Value> {
    if args.len() != 1 {
        bail!("dir() takes exactly 1 argument, got {}", args.len());
    }
    
    let path = match &args[0] {
        Value::String(s) => s.clone(),
        _ => bail!("dir() requires a string path"),
    };
    
    let entries = std::fs::read_dir(&path)
        .with_context(|| format!("Failed to read directory: {}", path))?;
    
    let files: Vec<Value> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| {
            let path = entry.path();
            Value::file_object(path.to_string_lossy().to_string())
        })
        .collect();
    
    Ok(Value::Array(files))
}

/// glob(pattern: string) -> Array<File>
/// Finds files matching a glob pattern
fn builtin_glob(args: &[Value]) -> Result<Value> {
    if args.len() != 1 {
        bail!("glob() takes exactly 1 argument, got {}", args.len());
    }
    
    let pattern = match &args[0] {
        Value::String(s) => s.clone(),
        _ => bail!("glob() requires a string pattern"),
    };
    
    let paths = glob::glob(&pattern)
        .with_context(|| format!("Invalid glob pattern: {}", pattern))?;
    
    let files: Vec<Value> = paths
        .filter_map(|path| path.ok())
        .map(|path| Value::file_object(path.to_string_lossy().to_string()))
        .collect();
    
    Ok(Value::Array(files))
}

/// exec(command: string) -> string
/// Executes a shell command and returns stdout
fn builtin_exec(args: &[Value]) -> Result<Value> {
    if args.len() != 1 {
        bail!("exec() takes exactly 1 argument, got {}", args.len());
    }
    
    let cmd = match &args[0] {
        Value::String(s) => s.clone(),
        _ => bail!("exec() requires a string command"),
    };
    
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .output()
        .with_context(|| format!("Failed to execute command: {}", cmd))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Command failed: {}", stderr);
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(Value::String(stdout))
}

/// http(url: string) -> HttpResponse
/// Makes an HTTP GET request
fn builtin_http(args: &[Value]) -> Result<Value> {
    if args.is_empty() || args.len() > 2 {
        bail!("http() takes 1-2 arguments, got {}", args.len());
    }
    
    let url = match &args[0] {
        Value::String(s) => s.clone(),
        _ => bail!("http() requires a string URL"),
    };
    
    // Optional: method (GET, POST, etc.)
    let method = if args.len() == 2 {
        match &args[1] {
            Value::String(s) => s.to_uppercase(),
            _ => "GET".to_string(),
        }
    } else {
        "GET".to_string()
    };
    
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("githook/1.0")
        .build()
        .context("Failed to create HTTP client")?;
    
    let response = match method.as_str() {
        "GET" => client.get(&url),
        "POST" => client.post(&url),
        "PUT" => client.put(&url),
        "DELETE" => client.delete(&url),
        _ => bail!("Unsupported HTTP method: {}", method),
    }
    .send()
    .with_context(|| format!("Failed to send HTTP request to {}", url))?;
    
    let status = response.status().as_u16();
    let headers: HashMap<String, String> = response
        .headers()
        .iter()
        .map(|(k, v)| {
            (
                k.as_str().to_lowercase(),
                v.to_str().unwrap_or("").to_string(),
            )
        })
        .collect();
    
    let body = response
        .text()
        .context("Failed to read response body")?;
    
    Ok(Value::http_response_object(status, body, headers))
}
