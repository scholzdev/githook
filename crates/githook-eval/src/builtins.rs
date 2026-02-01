use anyhow::{bail, Context, Result};
use crate::value::Value;
use std::collections::HashMap;
use once_cell::sync::Lazy;

pub type BuiltinFn = fn(&[Value]) -> Result<Value>;

static BUILTIN_FUNCTIONS: Lazy<HashMap<&'static str, BuiltinFn>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert("file", builtin_file as BuiltinFn);
    map.insert("dir", builtin_dir as BuiltinFn);
    map.insert("glob", builtin_glob as BuiltinFn);
    map.insert("exec", builtin_exec as BuiltinFn);
    map.insert("rm", bultin_rm as BuiltinFn);
    map
});

#[derive(Clone)]
pub struct BuiltinRegistry;

impl BuiltinRegistry {
    pub fn new() -> Self {
        Self
    }
    
    pub fn call(&self, name: &str, args: &[Value]) -> Result<Option<Value>> {
        if let Some(func) = BUILTIN_FUNCTIONS.get(name) {
            Ok(Some(func(args)?))
        } else {
            Ok(None)
        }
    }
    
    pub fn has(&self, name: &str) -> bool {
        BUILTIN_FUNCTIONS.contains_key(name)
    }
}

impl Default for BuiltinRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub fn builtin_file(args: &[Value]) -> Result<Value> {
    if args.len() != 1 {
        bail!("file() takes exactly 1 argument, got {}", args.len());
    }
    
    let path = match &args[0] {
        Value::String(s) => s.clone(),
        _ => bail!("file() requires a string path"),
    };
    
    Ok(Value::file_object(path))
}

pub fn builtin_dir(args: &[Value]) -> Result<Value> {
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

pub fn bultin_rm(args: &[Value]) -> Result<Value> {
    if args.len() != 1 {
        bail!("rm() takes exactly 1 argument, got {}", args.len());
    }
    let path = match &args[0] {
        Value::String(s) => s.clone(),
        _ => bail!("rm() requires a string path"),
    };
    std::fs::remove_file(&path)
        .with_context(|| format!("Failed to remove file: {}", path))?;
    
    Ok(Value::String(path))
}

pub fn builtin_glob(args: &[Value]) -> Result<Value> {
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

pub fn builtin_exec(args: &[Value]) -> Result<Value> {
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

pub fn builtin_http_get(args: &[Value]) -> Result<Value> {
    http_request("GET", args)
}

pub fn builtin_http_post(args: &[Value]) -> Result<Value> {
    http_request("POST", args)
}

pub fn builtin_http_put(args: &[Value]) -> Result<Value> {
    http_request("PUT", args)
}

pub fn builtin_http_delete(args: &[Value]) -> Result<Value> {
    http_request("DELETE", args)
}

fn http_request(method: &str, args: &[Value]) -> Result<Value> {
    if args.len() != 1 {
        bail!("http.{}() takes exactly 1 argument, got {}", method.to_lowercase(), args.len());
    }
    
    let url = match &args[0] {
        Value::String(s) => s.clone(),
        _ => bail!("http.{}() requires a string URL", method.to_lowercase()),
    };
    
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("githook/1.0")
        .build()
        .context("Failed to create HTTP client")?;
    
    let response = match method {
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
