use crate::value::Value;
use anyhow::{Context, Result, bail};
use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Type alias for a built-in function: takes arguments and returns a [`Value`].
pub type BuiltinFn = fn(&[Value]) -> Result<Value>;

static BUILTIN_FUNCTIONS: Lazy<HashMap<&'static str, BuiltinFn>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert("file", builtin_file as BuiltinFn);
    map.insert("dir", builtin_dir as BuiltinFn);
    map.insert("glob", builtin_glob as BuiltinFn);
    map.insert("exec", builtin_exec as BuiltinFn);
    map.insert("rm", builtin_rm as BuiltinFn);
    map
});

/// Registry of built-in functions (`file`, `dir`, `glob`, `exec`, `rm`).
///
/// Used by the [`Executor`](`crate::executor::Executor`) to resolve function calls
/// that are not user-defined macros.
#[derive(Clone)]
pub struct BuiltinRegistry;

impl BuiltinRegistry {
    /// Creates a new registry backed by the static function map.
    pub fn new() -> Self {
        Self
    }

    /// Calls a built-in function by name. Returns `Ok(None)` if not found.
    pub fn call(&self, name: &str, args: &[Value]) -> Result<Option<Value>> {
        if let Some(func) = BUILTIN_FUNCTIONS.get(name) {
            Ok(Some(func(args)?))
        } else {
            Ok(None)
        }
    }

    /// Returns `true` if a built-in function with the given name exists.
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

    let entries =
        std::fs::read_dir(&path).with_context(|| format!("Failed to read directory: {}", path))?;

    let files: Vec<Value> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| {
            let path = entry.path();
            Value::file_object(path.to_string_lossy().to_string())
        })
        .collect();

    Ok(Value::Array(files))
}

pub fn builtin_rm(args: &[Value]) -> Result<Value> {
    if args.len() != 1 {
        bail!("rm() takes exactly 1 argument, got {}", args.len());
    }
    let path = match &args[0] {
        Value::String(s) => s.clone(),
        _ => bail!("rm() requires a string path"),
    };
    std::fs::remove_file(&path).with_context(|| format!("Failed to remove file: {}", path))?;

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

    let paths =
        glob::glob(&pattern).with_context(|| format!("Invalid glob pattern: {}", pattern))?;

    let files: Vec<Value> = paths
        .filter_map(|path| path.ok())
        .map(|path| Value::file_object(path.to_string_lossy().to_string()))
        .collect();

    Ok(Value::Array(files))
}

/// Execute a shell command and return its stdout.
///
/// The command is run with a 30-second timeout. If the command does not
/// finish within that window it is killed and an error is returned.
pub fn builtin_exec(args: &[Value]) -> Result<Value> {
    if args.len() != 1 {
        bail!("exec() takes exactly 1 argument, got {}", args.len());
    }

    let cmd = match &args[0] {
        Value::String(s) => s.clone(),
        _ => bail!("exec() requires a string command"),
    };

    let mut child = std::process::Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("Failed to execute command: {}", cmd))?;

    let timeout = std::time::Duration::from_secs(30);
    let start = std::time::Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let output = child.wait_with_output()?;
                if !status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    bail!("Command failed: {}", stderr);
                }
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                return Ok(Value::String(stdout));
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    bail!(
                        "Command timed out after {} seconds: {}",
                        timeout.as_secs(),
                        cmd
                    );
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(e) => bail!("Error waiting for command: {}", e),
        }
    }
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
        bail!(
            "http.{}() takes exactly 1 argument, got {}",
            method.to_lowercase(),
            args.len()
        );
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

    let body = response.text().context("Failed to read response body")?;

    Ok(Value::http_response_object(status, body, headers))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_registry_has_known_builtins() {
        let reg = BuiltinRegistry::new();
        assert!(reg.has("file"));
        assert!(reg.has("dir"));
        assert!(reg.has("glob"));
        assert!(reg.has("exec"));
        assert!(reg.has("rm"));
    }

    #[test]
    fn test_registry_unknown_returns_false() {
        let reg = BuiltinRegistry::new();
        assert!(!reg.has("nonexistent"));
    }

    #[test]
    fn test_registry_call_unknown_returns_none() {
        let reg = BuiltinRegistry::new();
        let result = reg.call("nonexistent", &[]).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_file_wrong_arg_count() {
        let err = builtin_file(&[]).unwrap_err();
        assert!(err.to_string().contains("takes exactly 1 argument"));
    }

    #[test]
    fn test_file_wrong_arg_type() {
        let err = builtin_file(&[Value::Number(42.0)]).unwrap_err();
        assert!(err.to_string().contains("requires a string path"));
    }

    #[test]
    fn test_file_returns_object() {
        let result = builtin_file(&[Value::String("/tmp/test.txt".into())]).unwrap();
        assert!(matches!(result, Value::Object(_)));
    }

    #[test]
    fn test_dir_wrong_arg_count() {
        let err = builtin_dir(&[]).unwrap_err();
        assert!(err.to_string().contains("takes exactly 1 argument"));
    }

    #[test]
    fn test_dir_wrong_arg_type() {
        let err = builtin_dir(&[Value::Bool(true)]).unwrap_err();
        assert!(err.to_string().contains("requires a string path"));
    }

    #[test]
    fn test_glob_wrong_arg_count() {
        let err = builtin_glob(&[]).unwrap_err();
        assert!(err.to_string().contains("takes exactly 1 argument"));
    }

    #[test]
    fn test_glob_wrong_arg_type() {
        let err = builtin_glob(&[Value::Null]).unwrap_err();
        assert!(err.to_string().contains("requires a string pattern"));
    }

    #[test]
    fn test_glob_no_match_returns_empty_array() {
        let result =
            builtin_glob(&[Value::String("/tmp/nonexistent_githook_*.zzz".into())]).unwrap();
        assert!(matches!(result, Value::Array(arr) if arr.is_empty()));
    }

    #[test]
    fn test_exec_wrong_arg_count() {
        let err = builtin_exec(&[]).unwrap_err();
        assert!(err.to_string().contains("takes exactly 1 argument"));
    }

    #[test]
    fn test_exec_wrong_arg_type() {
        let err = builtin_exec(&[Value::Number(1.0)]).unwrap_err();
        assert!(err.to_string().contains("requires a string command"));
    }

    #[test]
    fn test_exec_simple_command() {
        let result = builtin_exec(&[Value::String("echo hello".into())]).unwrap();
        assert!(matches!(result, Value::String(s) if s == "hello\n"));
    }

    #[test]
    fn test_exec_failing_command() {
        let err = builtin_exec(&[Value::String("false".into())]).unwrap_err();
        assert!(err.to_string().contains("Command failed"));
    }

    #[test]
    fn test_rm_wrong_arg_count() {
        let err = builtin_rm(&[]).unwrap_err();
        assert!(err.to_string().contains("takes exactly 1 argument"));
    }

    #[test]
    fn test_rm_wrong_arg_type() {
        let err = builtin_rm(&[Value::Array(vec![])]).unwrap_err();
        assert!(err.to_string().contains("requires a string path"));
    }

    #[test]
    fn test_rm_nonexistent_file() {
        let err =
            builtin_rm(&[Value::String("/tmp/nonexistent_githook_rm_test".into())]).unwrap_err();
        assert!(err.to_string().contains("Failed to remove file"));
    }

    #[test]
    fn test_http_get_wrong_arg_count() {
        let err = builtin_http_get(&[]).unwrap_err();
        assert!(err.to_string().contains("takes exactly 1 argument"));
    }

    #[test]
    fn test_http_post_wrong_arg_type() {
        let err = builtin_http_post(&[Value::Number(42.0)]).unwrap_err();
        assert!(err.to_string().contains("requires a string URL"));
    }
}
