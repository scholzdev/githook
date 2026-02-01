use anyhow::{Result, bail};
use ahash::{HashMap, HashMapExt};
use crate::contexts::{
    StringContext, NumberContext, ArrayContext,
    FileContext, PathContext,
    GitContext, BranchInfo, CommitInfo, AuthorInfo, RemoteInfo, DiffStats, FilesCollection,
    HttpResponseContext, HttpContext,
};

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum Value {
    String(String),
    Number(f64),
    Bool(bool),
    Null,
    
    Array(Vec<Value>),
    Object(Object),
}

#[derive(Debug, Clone)]
pub struct Object {
    pub type_name: String,
    pub properties: HashMap<String, Value>,
    pub file_context: Option<FileContext>,
    pub path_context: Option<PathContext>,
    pub git_context: Option<GitContext>,
    pub files_context: Option<FilesCollection>,
    pub branch_context: Option<BranchInfo>,
    pub commit_context: Option<CommitInfo>,
    pub author_context: Option<AuthorInfo>,
    pub remote_context: Option<RemoteInfo>,
    pub stats_context: Option<DiffStats>,
    pub string_context: Option<StringContext>,
    pub number_context: Option<NumberContext>,
    pub array_context: Option<ArrayContext>,
    pub http_response_context: Option<HttpResponseContext>,
    pub http_context: Option<HttpContext>,
}

impl Object {
    pub fn new(type_name: impl Into<String>) -> Self {
        Self {
            type_name: type_name.into(),
            properties: HashMap::new(),
            file_context: None,
            path_context: None,
            git_context: None,
            files_context: None,
            branch_context: None,
            commit_context: None,
            author_context: None,
            remote_context: None,
            stats_context: None,
            string_context: None,
            number_context: None,
            array_context: None,
            http_response_context: None,
            http_context: None,
        }
    }
    
    pub fn with_property(mut self, key: impl Into<String>, value: Value) -> Self {
        self.properties.insert(key.into(), value);
        self
    }
    
    pub fn with_file_context(mut self, ctx: FileContext) -> Self {
        self.file_context = Some(ctx);
        self
    }
    
    pub fn with_path_context(mut self, ctx: PathContext) -> Self {
        self.path_context = Some(ctx);
        self
    }
    
    pub fn with_git_context(mut self, ctx: GitContext) -> Self {
        self.git_context = Some(ctx);
        self
    }
    
    pub fn with_files_context(mut self, ctx: FilesCollection) -> Self {
        self.files_context = Some(ctx);
        self
    }
    
    pub fn with_branch_context(mut self, ctx: BranchInfo) -> Self {
        self.branch_context = Some(ctx);
        self
    }
    
    pub fn with_commit_context(mut self, ctx: CommitInfo) -> Self {
        self.commit_context = Some(ctx);
        self
    }
    
    pub fn with_author_context(mut self, ctx: AuthorInfo) -> Self {
        self.author_context = Some(ctx);
        self
    }
    
    pub fn with_remote_context(mut self, ctx: RemoteInfo) -> Self {
        self.remote_context = Some(ctx);
        self
    }
    
    pub fn with_http_response_context(mut self, ctx: HttpResponseContext) -> Self {
        self.http_response_context = Some(ctx);
        self
    }
    
    pub fn with_http_context(mut self, ctx: HttpContext) -> Self {
        self.http_context = Some(ctx);
        self
    }
    
    pub fn with_stats_context(mut self, ctx: DiffStats) -> Self {
        self.stats_context = Some(ctx);
        self
    }
    
    pub fn with_string_context(mut self, ctx: StringContext) -> Self {
        self.string_context = Some(ctx);
        self
    }
    
    pub fn with_number_context(mut self, ctx: NumberContext) -> Self {
        self.number_context = Some(ctx);
        self
    }
    
    pub fn with_array_context(mut self, ctx: ArrayContext) -> Self {
        self.array_context = Some(ctx);
        self
    }
    
    #[inline]
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.properties.get(key)
    }
    
    #[inline]
    pub fn set(&mut self, key: impl Into<String>, value: Value) {
        self.properties.insert(key.into(), value);
    }
}

impl Value {
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
    
    #[inline]
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }
    
    #[inline]
    pub fn is_number(&self) -> bool {
        matches!(self, Value::Number(_))
    }
    
    #[inline]
    pub fn is_bool(&self) -> bool {
        matches!(self, Value::Bool(_))
    }
    
    #[inline]
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }
    
    #[inline]
    pub fn is_array(&self) -> bool {
        matches!(self, Value::Array(_))
    }
    
    #[inline]
    pub fn is_object(&self) -> bool {
        matches!(self, Value::Object(_))
    }
    
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
    
    pub fn get_property(&self, name: &str) -> Result<Value> {
        match self {
            Value::Object(obj) => {
                if obj.type_name == "File"
                    && let Some(ctx) = &obj.file_context
                        && let Ok(value) = ctx.call_property(name) {
                            return Ok(value);
                        }
                if obj.type_name == "Path"
                    && let Some(ctx) = &obj.path_context
                        && let Ok(value) = ctx.call_property(name) {
                            return Ok(value);
                        }
                if obj.type_name == "Git"
                    && let Some(ctx) = &obj.git_context
                        && let Ok(value) = ctx.call_property(name) {
                            return Ok(value);
                        }
                if obj.type_name == "FilesCollection"
                    && let Some(ctx) = &obj.files_context {
                        match name {
                            "staged" => {
                                let files: Vec<Value> = ctx.staged().iter()
                                    .map(|path| Value::file_object(path.clone()))
                                    .collect();
                                return Ok(Value::Array(files));
                            }
                            "all" => {
                                let files: Vec<Value> = ctx.all().iter()
                                    .map(|path| Value::file_object(path.clone()))
                                    .collect();
                                return Ok(Value::Array(files));
                            }
                            "modified" => {
                                let files: Vec<Value> = ctx.modified().iter()
                                    .map(|path| Value::file_object(path.clone()))
                                    .collect();
                                return Ok(Value::Array(files));
                            }
                            "added" => {
                                let files: Vec<Value> = ctx.added().iter()
                                    .map(|path| Value::file_object(path.clone()))
                                    .collect();
                                return Ok(Value::Array(files));
                            }
                            "deleted" => {
                                let files: Vec<Value> = ctx.deleted().iter()
                                    .map(|path| Value::file_object(path.clone()))
                                    .collect();
                                return Ok(Value::Array(files));
                            }
                            "unstaged" => {
                                let files: Vec<Value> = ctx.unstaged().iter()
                                    .map(|path| Value::file_object(path.clone()))
                                    .collect();
                                return Ok(Value::Array(files));
                            }
                            _ => {}
                        }
                    }
                if obj.type_name == "Branch"
                    && let Some(ctx) = &obj.branch_context
                        && let Ok(value) = ctx.call_property(name) {
                            return Ok(value);
                        }
                if obj.type_name == "Commit"
                    && let Some(ctx) = &obj.commit_context
                        && let Ok(value) = ctx.call_property(name) {
                            return Ok(value);
                        }
                if obj.type_name == "Author"
                    && let Some(ctx) = &obj.author_context
                        && let Ok(value) = ctx.call_property(name) {
                            return Ok(value);
                        }
                if obj.type_name == "Remote"
                    && let Some(ctx) = &obj.remote_context
                        && let Ok(value) = ctx.call_property(name) {
                            return Ok(value);
                        }
                if obj.type_name == "Stats"
                    && let Some(ctx) = &obj.stats_context
                        && let Ok(value) = ctx.call_property(name) {
                            return Ok(value);
                        }
                if obj.type_name == "String"
                    && let Some(ctx) = &obj.string_context
                        && let Ok(value) = ctx.call_property(name) {
                            return Ok(value);
                        }
                if obj.type_name == "Number"
                    && let Some(ctx) = &obj.number_context
                        && let Ok(value) = ctx.call_property(name) {
                            return Ok(value);
                        }
                if obj.type_name == "Array"
                    && let Some(ctx) = &obj.array_context
                        && let Ok(value) = ctx.call_property(name) {
                            return Ok(value);
                        }
                if obj.type_name == "HttpResponse"
                    && let Some(ctx) = &obj.http_response_context
                        && let Ok(value) = ctx.call_property(name) {
                            return Ok(value);
                        }
                obj.get(name)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("Property '{}' not found on {}", name, obj.type_name))
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

    #[allow(dead_code)]
    pub fn call_method(&self, name: &str, args: &[Value]) -> Result<Value> {
        match self {
            Value::String(s) => {
                let ctx = StringContext::new(s.clone());
                let string_args: Result<Vec<&str>> = args.iter().map(|v| {
                    v.as_string().map(|s| Box::leak(s.into_boxed_str()) as &str)
                }).collect();
                ctx.call_method(name, &string_args?)
            }
            Value::Number(n) => {
                let ctx = NumberContext::new(*n);
                let string_args: Result<Vec<&str>> = args.iter().map(|v| {
                    v.as_string().map(|s| Box::leak(s.into_boxed_str()) as &str)
                }).collect();
                ctx.call_method(name, &string_args?)
            }
            Value::Array(arr) => {
                let ctx = ArrayContext::new(arr.clone());
                let string_args: Result<Vec<&str>> = args.iter().map(|v| {
                    v.as_string().map(|s| Box::leak(s.into_boxed_str()) as &str)
                }).collect();
                ctx.call_method(name, &string_args?)
            }
            Value::Object(obj) => self.call_object_method(obj, name, args),
            _ => bail!("Cannot call method '{}' on {:?}", name, self),
        }
    }
    
    fn call_object_method(&self, obj: &Object, name: &str, args: &[Value]) -> Result<Value> {
        if obj.type_name == "File" {
            if let Some(ctx) = &obj.file_context {
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
        else if obj.type_name == "Path" {
            if let Some(ctx) = &obj.path_context {
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
        else if obj.type_name == "HttpResponse" {
            if let Some(ctx) = &obj.http_response_context {
                if name == "json" && args.is_empty() {
                    return Ok(ctx.json_parsed());
                }
                
                let string_args: Result<Vec<String>> = args.iter()
                    .map(|v| v.as_string())
                    .collect();
                let string_args = string_args?;
                let str_refs: Vec<&str> = string_args.iter().map(|s| s.as_str()).collect();
                ctx.call_method(name, &str_refs)
            } else {
                bail!("HttpResponse object has no context")
            }
        }
        else if obj.type_name == "Http" {
            if obj.http_context.is_some() {
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
            } else {
                bail!("Http object has no context")
            }
        } 
        else {
            bail!("Method '{}' not found on {}", name, obj.type_name)
        }
    }
    
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
                if let Some(path_ctx) = &obj.path_context {
                    return path_ctx.to_string();
                }
                format!("{}{{ {} properties }}", obj.type_name, obj.properties.len())
            }
        }
    }
}

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

impl Value {
    pub fn file_object(path: String) -> Value {
        let file_ctx = FileContext::from_path(&path);
        
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
        let path_ctx = PathContext::from_path(&path);
        Value::Object(
            Object::new("Path")
                .with_path_context(path_ctx)
        )
    }
    
    pub fn http_response_object(status: u16, body: String, headers: std::collections::HashMap<String, String>) -> Value {
        let response_ctx = crate::contexts::HttpResponseContext::new(status, body, headers);
        Value::Object(
            Object::new("HttpResponse")
                .with_http_response_context(response_ctx)
        )
    }
    
    pub fn http_object() -> Value {
        let http_ctx = crate::contexts::HttpContext::new();
        Value::Object(
            Object::new("Http")
                .with_http_context(http_ctx)
        )
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
