use crate::contexts::{
    ArrayContext, AuthorInfo, BranchInfo, CommitInfo, DiffStats, FileContext, FilesCollection,
    GitContext, HttpContext, HttpResponseContext, NumberContext, PathContext, RemoteInfo,
    StringContext,
};
use ahash::{HashMap, HashMapExt};
use anyhow::{Result, bail};

/// Typed context attached to an [`Object`], providing domain-specific
/// property and method dispatch.
///
/// Each object carries at most one context. The executor matches on this
/// enum to resolve property accesses and method calls.
#[derive(Debug, Clone)]
pub enum Context {
    /// File I/O context.
    File(FileContext),
    /// Path helper context.
    Path(PathContext),
    /// Root Git context (`git.*`), boxed to reduce enum size.
    Git(Box<GitContext>),
    /// Files collection context (`git.files.*`).
    FilesCollection(FilesCollection),
    /// Branch info context (`git.branch.*`).
    Branch(BranchInfo),
    /// Commit info context (`git.commit.*`).
    Commit(CommitInfo),
    /// Author info context (`git.author.*`).
    Author(AuthorInfo),
    /// Remote info context (`git.remote.*`).
    Remote(RemoteInfo),
    /// Diff statistics context (`git.stats.*`).
    Stats(DiffStats),
    /// String methods context.
    String(StringContext),
    /// Number methods context.
    Number(NumberContext),
    /// Array methods context.
    Array(ArrayContext),
    /// HTTP response context.
    HttpResponse(HttpResponseContext),
    /// HTTP client context.
    Http(HttpContext),
}

/// A runtime value in the Githook scripting language.
///
/// All expressions evaluate to a `Value`. The type system is dynamically
/// typed — conversions happen at runtime (see [`as_string`](`Value::as_string`),
/// [`as_number`](`Value::as_number`), [`is_truthy`](`Value::is_truthy`)).
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum Value {
    /// A UTF-8 string.
    String(String),
    /// A 64-bit floating point number.
    Number(f64),
    /// A boolean (`true`/`false`).
    Bool(bool),
    /// The `null` value.
    Null,
    /// An ordered list of values.
    Array(Vec<Value>),
    /// A named object with typed context.
    Object(Object),
}

/// A named object that carries dynamic properties and an optional typed context.
///
/// Objects represent the Git model (`git`, `git.branch`, files, etc.) and
/// user-created objects. The `type_name` distinguishes them, and the
/// [`context`](`Object::context`) provides typed access to domain-specific methods.
#[derive(Debug, Clone)]
pub struct Object {
    /// The type name (e.g. `"Git"`, `"File"`, `"String"`).
    pub type_name: String,
    /// Dynamic key-value properties.
    pub properties: HashMap<String, Value>,
    /// Optional typed context for property/method dispatch.
    pub context: Option<Context>,
}

impl Object {
    /// Creates a new object with the given type name and no properties.
    pub fn new(type_name: impl Into<String>) -> Self {
        Self {
            type_name: type_name.into(),
            properties: HashMap::new(),
            context: None,
        }
    }

    /// Builder: adds a dynamic property.
    pub fn with_property(mut self, key: impl Into<String>, value: Value) -> Self {
        self.properties.insert(key.into(), value);
        self
    }

    /// Builder: sets the typed context.
    pub fn with_context(mut self, ctx: Context) -> Self {
        self.context = Some(ctx);
        self
    }

    pub fn with_file_context(mut self, ctx: FileContext) -> Self {
        self.context = Some(Context::File(ctx));
        self
    }

    pub fn with_path_context(mut self, ctx: PathContext) -> Self {
        self.context = Some(Context::Path(ctx));
        self
    }

    pub fn with_git_context(mut self, ctx: GitContext) -> Self {
        self.context = Some(Context::Git(Box::new(ctx)));
        self
    }

    pub fn with_files_context(mut self, ctx: FilesCollection) -> Self {
        self.context = Some(Context::FilesCollection(ctx));
        self
    }

    pub fn with_branch_context(mut self, ctx: BranchInfo) -> Self {
        self.context = Some(Context::Branch(ctx));
        self
    }

    pub fn with_commit_context(mut self, ctx: CommitInfo) -> Self {
        self.context = Some(Context::Commit(ctx));
        self
    }

    pub fn with_author_context(mut self, ctx: AuthorInfo) -> Self {
        self.context = Some(Context::Author(ctx));
        self
    }

    pub fn with_remote_context(mut self, ctx: RemoteInfo) -> Self {
        self.context = Some(Context::Remote(ctx));
        self
    }

    pub fn with_http_response_context(mut self, ctx: HttpResponseContext) -> Self {
        self.context = Some(Context::HttpResponse(ctx));
        self
    }

    pub fn with_http_context(mut self, ctx: HttpContext) -> Self {
        self.context = Some(Context::Http(ctx));
        self
    }

    pub fn with_stats_context(mut self, ctx: DiffStats) -> Self {
        self.context = Some(Context::Stats(ctx));
        self
    }

    pub fn with_string_context(mut self, ctx: StringContext) -> Self {
        self.context = Some(Context::String(ctx));
        self
    }

    pub fn with_number_context(mut self, ctx: NumberContext) -> Self {
        self.context = Some(Context::Number(ctx));
        self
    }

    pub fn with_array_context(mut self, ctx: ArrayContext) -> Self {
        self.context = Some(Context::Array(ctx));
        self
    }

    /// Looks up a dynamic property by name.
    #[inline]
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.properties.get(key)
    }

    /// Sets (or overwrites) a dynamic property.
    #[inline]
    pub fn set(&mut self, key: impl Into<String>, value: Value) {
        self.properties.insert(key.into(), value);
    }
}

impl Value {
    /// Creates an `env` object populated with common environment variables.
    pub fn env_object() -> Self {
        let mut obj = Object::new("env");

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

    /// Returns `true` if this value is a string.
    #[inline]
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    /// Returns `true` if this value is a number.
    #[inline]
    pub fn is_number(&self) -> bool {
        matches!(self, Value::Number(_))
    }

    /// Returns `true` if this value is a boolean.
    #[inline]
    pub fn is_bool(&self) -> bool {
        matches!(self, Value::Bool(_))
    }

    /// Returns `true` if this value is null.
    #[inline]
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Returns `true` if this value is an array.
    #[inline]
    pub fn is_array(&self) -> bool {
        matches!(self, Value::Array(_))
    }

    /// Returns `true` if this value is an object.
    #[inline]
    pub fn is_object(&self) -> bool {
        matches!(self, Value::Object(_))
    }

    /// Returns `true` if this value is considered "truthy".
    ///
    /// Truthiness rules: `false`, `null`, empty strings, zero, and empty
    /// arrays are falsy. Everything else is truthy.
    #[inline]
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

    /// Converts this value to a `String`, coercing numbers and bools.
    pub fn as_string(&self) -> Result<String> {
        match self {
            Value::String(s) => Ok(s.clone()),
            Value::Number(n) => Ok(n.to_string()),
            Value::Bool(b) => Ok(b.to_string()),
            Value::Null => Ok("null".to_string()),
            _ => bail!("Cannot convert {:?} to string", self),
        }
    }

    /// Converts this value to an `f64`. Strings are parsed.
    pub fn as_number(&self) -> Result<f64> {
        match self {
            Value::Number(n) => Ok(*n),
            Value::String(s) => s
                .parse()
                .map_err(|_| anyhow::anyhow!("Cannot parse '{}' as number", s)),
            _ => bail!("Cannot convert {:?} to number", self),
        }
    }

    /// Converts this value to a `bool` using truthiness rules.
    pub fn as_bool(&self) -> Result<bool> {
        match self {
            Value::Bool(b) => Ok(*b),
            _ => Ok(self.is_truthy()),
        }
    }

    /// Resolves a property access (`value.name`) by delegating to the
    /// appropriate typed context or falling back to dynamic properties.
    pub fn get_property(&self, name: &str) -> Result<Value> {
        match self {
            Value::Object(obj) => {
                if let Some(ctx) = &obj.context {
                    match ctx {
                        Context::File(c) => {
                            if let Ok(v) = c.call_property(name) {
                                return Ok(v);
                            }
                        }
                        Context::Path(c) => {
                            if let Ok(v) = c.call_property(name) {
                                return Ok(v);
                            }
                        }
                        Context::Git(c) => {
                            if let Ok(v) = c.call_property(name) {
                                return Ok(v);
                            }
                        }
                        Context::FilesCollection(c) => {
                            let accessor = |items: &[String]| -> Value {
                                Value::Array(
                                    items
                                        .iter()
                                        .map(|p| Value::file_object(p.clone()))
                                        .collect(),
                                )
                            };
                            match name {
                                "staged" => return Ok(accessor(&c.staged)),
                                "all" => return Ok(accessor(&c.all)),
                                "modified" => return Ok(accessor(&c.modified)),
                                "added" => return Ok(accessor(&c.added)),
                                "deleted" => return Ok(accessor(&c.deleted)),
                                "unstaged" => return Ok(accessor(&c.unstaged)),
                                _ => {}
                            }
                        }
                        Context::Branch(c) => {
                            if let Ok(v) = c.call_property(name) {
                                return Ok(v);
                            }
                        }
                        Context::Commit(c) => {
                            if let Ok(v) = c.call_property(name) {
                                return Ok(v);
                            }
                        }
                        Context::Author(c) => {
                            if let Ok(v) = c.call_property(name) {
                                return Ok(v);
                            }
                        }
                        Context::Remote(c) => {
                            if let Ok(v) = c.call_property(name) {
                                return Ok(v);
                            }
                        }
                        Context::Stats(c) => {
                            if let Ok(v) = c.call_property(name) {
                                return Ok(v);
                            }
                        }
                        Context::String(c) => {
                            if let Ok(v) = c.call_property(name) {
                                return Ok(v);
                            }
                        }
                        Context::Number(c) => {
                            if let Ok(v) = c.call_property(name) {
                                return Ok(v);
                            }
                        }
                        Context::Array(c) => {
                            if let Ok(v) = c.call_property(name) {
                                return Ok(v);
                            }
                        }
                        Context::HttpResponse(c) => {
                            if name == "json" {
                                return Ok(c.json_parsed());
                            }
                            if let Ok(v) = c.call_property(name) {
                                return Ok(v);
                            }
                        }
                        Context::Http(_) => {}
                    }
                }
                obj.get(name).cloned().ok_or_else(|| {
                    anyhow::anyhow!("Property '{}' not found on {}", name, obj.type_name)
                })
            }
            Value::String(s) => {
                let ctx = StringContext::new(s.clone());
                ctx.call_property(name)
            }
            Value::Number(n) => {
                let ctx = NumberContext::new(*n);
                ctx.call_property(name)
            }
            Value::Array(arr) => {
                let ctx = ArrayContext::new(arr.clone());
                ctx.call_property(name)
            }
            _ => bail!("Cannot access property '{}' on {:?}", name, self),
        }
    }

    /// Calls a method on this value (e.g. `string.contains("x")`).
    ///
    /// Delegates to the appropriate typed context.
    #[allow(dead_code)]
    pub fn call_method(&self, name: &str, args: &[Value]) -> Result<Value> {
        match self {
            Value::String(s) => {
                let ctx = StringContext::new(s.clone());
                let owned_args: Result<Vec<String>> = args.iter().map(|v| v.as_string()).collect();
                let owned_args = owned_args?;
                let str_refs: Vec<&str> = owned_args.iter().map(|s| s.as_str()).collect();
                ctx.call_method(name, &str_refs)
            }
            Value::Number(n) => {
                let ctx = NumberContext::new(*n);
                let owned_args: Result<Vec<String>> = args.iter().map(|v| v.as_string()).collect();
                let owned_args = owned_args?;
                let str_refs: Vec<&str> = owned_args.iter().map(|s| s.as_str()).collect();
                ctx.call_method(name, &str_refs)
            }
            Value::Array(arr) => {
                // Special handling for first() and last() to return the actual Value
                if name == "first" && args.is_empty() {
                    return Ok(arr.first().cloned().unwrap_or(Value::Null));
                }
                if name == "last" && args.is_empty() {
                    return Ok(arr.last().cloned().unwrap_or(Value::Null));
                }

                let ctx = ArrayContext::new(arr.clone());
                let owned_args: Result<Vec<String>> = args.iter().map(|v| v.as_string()).collect();
                let owned_args = owned_args?;
                let str_refs: Vec<&str> = owned_args.iter().map(|s| s.as_str()).collect();
                ctx.call_method(name, &str_refs)
            }
            Value::Object(obj) => self.call_object_method(obj, name, args),
            _ => bail!("Cannot call method '{}' on {:?}", name, self),
        }
    }

    fn call_object_method(&self, obj: &Object, name: &str, args: &[Value]) -> Result<Value> {
        match &obj.context {
            Some(Context::File(ctx)) => {
                let owned: Vec<String> =
                    args.iter().map(|v| v.as_string()).collect::<Result<_>>()?;
                let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
                ctx.call_method(name, &refs)
            }
            Some(Context::Path(ctx)) => {
                let owned: Vec<String> =
                    args.iter().map(|v| v.as_string()).collect::<Result<_>>()?;
                let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
                ctx.call_method(name, &refs)
            }
            Some(Context::HttpResponse(ctx)) => {
                if name == "json" && args.is_empty() {
                    return Ok(ctx.json_parsed());
                }
                let owned: Vec<String> =
                    args.iter().map(|v| v.as_string()).collect::<Result<_>>()?;
                let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
                ctx.call_method(name, &refs)
            }
            Some(Context::Http(_)) => {
                if args.len() != 1 {
                    bail!("http.{}() takes exactly 1 argument (url)", name);
                }
                let url = args[0].as_string()?;
                match name {
                    "get" => crate::builtins::builtin_http_get(&[Value::String(url)]),
                    "post" => crate::builtins::builtin_http_post(&[Value::String(url)]),
                    "put" => crate::builtins::builtin_http_put(&[Value::String(url)]),
                    "delete" => crate::builtins::builtin_http_delete(&[Value::String(url)]),
                    _ => bail!("Method 'http.{}' not found", name),
                }
            }
            _ => bail!("Method '{}' not found on {}", name, obj.type_name),
        }
    }

    /// Returns a human-readable string representation of this value
    /// (used by the `print` statement).
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
                items.join(", ")
            }
            Value::Object(obj) => {
                if let Some(Context::Path(path_ctx)) = &obj.context {
                    return path_ctx.to_string();
                }
                format!("{}{{ {} properties }}", obj.type_name, obj.properties.len())
            }
        }
    }
}

impl Value {
    /// Tests equality between two values (the `==` operator).
    pub fn equals(&self, other: &Value) -> Result<bool> {
        match (self, other) {
            (Value::String(a), Value::String(b)) => Ok(a == b),
            (Value::Number(a), Value::Number(b)) => Ok((a - b).abs() < f64::EPSILON),
            (Value::Bool(a), Value::Bool(b)) => Ok(a == b),
            (Value::Null, Value::Null) => Ok(true),
            _ => Ok(false),
        }
    }

    /// Tests inequality (`!=`).
    pub fn not_equals(&self, other: &Value) -> Result<bool> {
        Ok(!self.equals(other)?)
    }

    /// Numeric less-than comparison (`<`).
    pub fn less_than(&self, other: &Value) -> Result<bool> {
        let a = self.as_number()?;
        let b = other.as_number()?;
        Ok(a < b)
    }

    /// Numeric less-or-equal comparison (`<=`).
    pub fn less_or_equal(&self, other: &Value) -> Result<bool> {
        let a = self.as_number()?;
        let b = other.as_number()?;
        Ok(a <= b)
    }

    /// Numeric greater-than comparison (`>`).
    pub fn greater_than(&self, other: &Value) -> Result<bool> {
        let a = self.as_number()?;
        let b = other.as_number()?;
        Ok(a > b)
    }

    /// Numeric greater-or-equal comparison (`>=`).
    pub fn greater_or_equal(&self, other: &Value) -> Result<bool> {
        let a = self.as_number()?;
        let b = other.as_number()?;
        Ok(a >= b)
    }
}

impl Value {
    /// Creates a `File` object from a filesystem path.
    pub fn file_object(path: String) -> Value {
        let file_ctx = FileContext::from_path(&path);

        let path_obj = Value::Object(Object::new("Path").with_path_context(file_ctx.path.clone()));

        Value::Object(
            Object::new("File")
                .with_property("path", path_obj)
                .with_file_context(file_ctx),
        )
    }

    /// Creates a `Path` object from a filesystem path.
    pub fn path_object(path: String) -> Value {
        let path_ctx = PathContext::from_path(&path);
        Value::Object(Object::new("Path").with_path_context(path_ctx))
    }

    /// Creates an `HttpResponse` object from a status code, body, and headers.
    pub fn http_response_object(
        status: u16,
        body: String,
        headers: std::collections::HashMap<String, String>,
    ) -> Value {
        let response_ctx = crate::contexts::HttpResponseContext::new(status, body, headers);
        Value::Object(Object::new("HttpResponse").with_http_response_context(response_ctx))
    }

    /// Creates the root `http` object for HTTP operations.
    pub fn http_object() -> Value {
        let http_ctx = crate::contexts::HttpContext::new();
        Value::Object(Object::new("Http").with_http_context(http_ctx))
    }
}

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

impl From<Vec<String>> for Value {
    fn from(arr: Vec<String>) -> Self {
        Value::Array(arr.into_iter().map(Value::String).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_types() {
        let string_val = Value::String("hello".to_string());
        let number_val = Value::Number(42.0);
        let bool_val = Value::Bool(true);
        let null_val = Value::Null;
        let array_val = Value::Array(vec![Value::Number(1.0)]);
        let object_val = Value::Object(Object::new("Test"));

        assert!(string_val.is_string());
        assert!(number_val.is_number());
        assert!(bool_val.is_bool());
        assert!(null_val.is_null());
        assert!(array_val.is_array());
        assert!(object_val.is_object());
    }

    #[test]
    fn test_is_truthy() {
        assert!(Value::Bool(true).is_truthy());
        assert!(!Value::Bool(false).is_truthy());
        assert!(!Value::Null.is_truthy());
        assert!(Value::String("hello".to_string()).is_truthy());
        assert!(!Value::String("".to_string()).is_truthy());
        assert!(Value::Number(1.0).is_truthy());
        assert!(!Value::Number(0.0).is_truthy());
        assert!(Value::Array(vec![Value::Number(1.0)]).is_truthy());
        assert!(!Value::Array(vec![]).is_truthy());
    }

    #[test]
    fn test_as_string() {
        assert_eq!(
            Value::String("test".to_string()).as_string().unwrap(),
            "test"
        );
        assert_eq!(Value::Number(42.0).as_string().unwrap(), "42");
        assert_eq!(Value::Bool(true).as_string().unwrap(), "true");
        assert_eq!(Value::Null.as_string().unwrap(), "null");
    }

    #[test]
    fn test_as_number() {
        assert_eq!(Value::Number(3.15).as_number().unwrap(), 3.15);
        assert_eq!(Value::String("42".to_string()).as_number().unwrap(), 42.0);
        assert!(
            Value::String("not a number".to_string())
                .as_number()
                .is_err()
        );
    }

    #[test]
    fn test_as_bool() {
        assert!(Value::Bool(true).as_bool().unwrap());
        assert!(!Value::Bool(false).as_bool().unwrap());
        assert!(Value::Number(1.0).as_bool().unwrap());
        assert!(!Value::Number(0.0).as_bool().unwrap());
        assert!(Value::String("hi".to_string()).as_bool().unwrap());
        assert!(!Value::String("".to_string()).as_bool().unwrap());
    }

    #[test]
    fn test_from_conversions() {
        let bool_val: Value = true.into();
        assert!(matches!(bool_val, Value::Bool(true)));

        let string_val: Value = "hello".to_string().into();
        assert!(matches!(string_val, Value::String(_)));

        let number_val: Value = 42.0.into();
        assert!(matches!(number_val, Value::Number(42.0)));

        let array_val: Value = vec!["a".to_string(), "b".to_string()].into();
        assert!(matches!(array_val, Value::Array(_)));
    }

    #[test]
    fn test_object_new() {
        let obj = Object::new("TestType");
        assert_eq!(obj.type_name, "TestType");
        assert!(obj.properties.is_empty());
    }

    #[test]
    fn test_object_with_property() {
        let obj = Object::new("Test")
            .with_property("name", Value::String("John".to_string()))
            .with_property("age", Value::Number(30.0));

        assert_eq!(obj.properties.len(), 2);
        assert!(matches!(obj.properties.get("name"), Some(Value::String(_))));
        assert!(matches!(
            obj.properties.get("age"),
            Some(Value::Number(30.0))
        ));
    }

    #[test]
    fn test_object_get_property() {
        let mut obj = Object::new("Test");
        obj.set("name", Value::String("Alice".to_string()));

        assert!(obj.get("name").is_some());
        if let Some(Value::String(s)) = obj.get("name") {
            assert_eq!(s, "Alice");
        }
        assert!(obj.get("missing").is_none());
    }

    #[test]
    fn test_value_array_operations() {
        let arr = Value::Array(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
        ]);

        assert!(arr.is_array());
        assert!(arr.is_truthy());

        if let Value::Array(items) = arr {
            assert_eq!(items.len(), 3);
            assert!(matches!(items[0], Value::Number(1.0)));
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_value_equality() {
        let val1 = Value::String("test".to_string());
        let val2 = Value::String("test".to_string());
        let val3 = Value::String("other".to_string());

        assert_eq!(val1.as_string().unwrap(), val2.as_string().unwrap());
        assert_ne!(val1.as_string().unwrap(), val3.as_string().unwrap());
    }

    #[test]
    fn test_number_edge_cases() {
        assert!(Value::Number(f64::NAN).is_number());
        assert!(Value::Number(f64::INFINITY).is_number());
        assert!(Value::Number(f64::NEG_INFINITY).is_number());
        assert!(Value::Number(f64::NAN).is_truthy());
    }

    #[test]
    fn test_empty_values() {
        assert!(!Value::String("".to_string()).is_truthy());
        assert!(!Value::Array(vec![]).is_truthy());
        assert!(!Value::Number(0.0).is_truthy());
    }
}
