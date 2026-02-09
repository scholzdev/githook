//! HTTP context types (request client, response wrapper).

use githook_macros::{callable_impl, docs};

/// Marker context for the `http` built-in object.
#[derive(Debug, Clone, Default)]
pub struct HttpContext {
    /// Timeout for HTTP requests.
    pub timeout_secs: u64,
    /// Optional bearer token for authentication.
    pub auth_token: Option<String>,
}

impl HttpContext {
    /// Creates a new HTTP context with default settings.
    pub fn new() -> Self {
        Self {
            timeout_secs: 30,
            auth_token: None,
        }
    }

    /// Creates an HTTP context with the given timeout and auth token.
    pub fn with_config(timeout_secs: u64, auth_token: Option<String>) -> Self {
        Self {
            timeout_secs,
            auth_token,
        }
    }
}

/// Typed context wrapping an HTTP response (status, body, headers).
#[derive(Debug, Clone)]
pub struct HttpResponseContext {
    status: u16,
    body: String,
    headers: std::collections::HashMap<String, String>,
}

impl HttpResponseContext {
    /// Wraps a raw HTTP response.
    pub fn new(
        status: u16,
        body: String,
        headers: std::collections::HashMap<String, String>,
    ) -> Self {
        Self {
            status,
            body,
            headers,
        }
    }
}

#[callable_impl]
impl HttpResponseContext {
    #[docs(
        name = "response.status",
        description = "HTTP status code",
        example = "if response.status == 200 { print \"OK\" }"
    )]
    #[property]
    pub fn status(&self) -> f64 {
        self.status as f64
    }

    #[docs(
        name = "response.body",
        description = "Response body as string",
        example = "print response.body"
    )]
    #[property]
    pub fn body(&self) -> String {
        self.body.clone()
    }

    #[docs(
        name = "response.ok",
        description = "Whether status is 2xx",
        example = "if response.ok { print \"Success\" }"
    )]
    #[property]
    pub fn ok(&self) -> bool {
        self.status >= 200 && self.status < 300
    }

    #[docs(
        name = "response.header",
        description = "Get response header by name",
        example = "print response.header(\"content-type\")"
    )]
    #[method]
    pub fn header(&self, name: &str) -> String {
        self.headers
            .get(&name.to_lowercase())
            .cloned()
            .unwrap_or_default()
    }
}

impl HttpResponseContext {
    /// Parses the response body as JSON and returns a `Value`.
    pub fn json_parsed(&self) -> crate::value::Value {
        match serde_json::from_str::<serde_json::Value>(&self.body) {
            Ok(parsed) => json_to_value(parsed),
            Err(_) => crate::value::Value::Null,
        }
    }
}

/// Converts a `serde_json::Value` into a githook `Value`.
pub fn json_to_value(json: serde_json::Value) -> crate::value::Value {
    use crate::value::Value;
    match json {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Number(i as f64)
            } else if let Some(f) = n.as_f64() {
                Value::Number(f)
            } else {
                Value::Null
            }
        }
        serde_json::Value::String(s) => Value::String(s),
        serde_json::Value::Array(arr) => Value::Array(arr.into_iter().map(json_to_value).collect()),
        serde_json::Value::Object(obj) => {
            let mut dict = crate::value::Object::new("Dict");
            for (k, v) in obj {
                dict.set(&k, json_to_value(v));
            }
            Value::Object(dict)
        }
    }
}
