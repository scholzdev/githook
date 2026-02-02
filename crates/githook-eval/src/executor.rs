use anyhow::{Context as _, Result, bail};
use rustc_hash::FxHashMap;
use std::process::Command;
use rayon::prelude::*;

use crate::value::{Value, Object};
use crate::contexts::GitContext;
use crate::builtins::BuiltinRegistry;
use crate::control_flow::ExecutionResult;
use githook_syntax::ast::{Statement, Expression, BinaryOp, UnaryOp, MatchPattern, Severity};

type VariableMap = FxHashMap<String, Value>;
type MacroMap = FxHashMap<String, (Vec<String>, Vec<Statement>)>;

#[derive(Debug, Clone)]
pub enum CheckStatus {
    Passed,
    Skipped,
    Failed,
}

#[derive(Debug, Clone)]
pub struct CheckResult {
    pub name: String,
    pub status: CheckStatus,
    pub reason: Option<String>,
    pub severity: Severity,
}

#[derive(Clone)]
pub struct Executor {
    pub variables: VariableMap,
    git_files: Vec<String>,
    pub verbose: bool,
    pub warnings: Vec<String>,
    pub blocks: Vec<String>,
    pub tests_run: usize,
    macros: MacroMap,
    namespaced_macros: MacroMap,
    pub check_results: Vec<CheckResult>,
    builtins: BuiltinRegistry,
}

impl Executor {
    pub fn new() -> Self {
        Self {
            variables: FxHashMap::default(),
            git_files: Vec::new(),
            verbose: false,
            warnings: Vec::new(),
            blocks: Vec::new(),
            tests_run: 0,
            macros: FxHashMap::default(),
            namespaced_macros: FxHashMap::default(),
            check_results: Vec::new(),
            builtins: BuiltinRegistry::new(),
        }
    }
    
    pub fn with_git_files(mut self, files: Vec<String>) -> Self {
        self.git_files = files;
        self
    }
    
    pub fn set_variable(&mut self, name: String, value: Value) {
        self.variables.insert(name, value);
    }
    
    pub fn add_check(&mut self, name: String, status: CheckStatus, reason: Option<String>, severity: Severity) {
        self.check_results.push(CheckResult { name, status, reason, severity });
    }
    
    pub fn eval_expression(&self, expr: &Expression) -> Result<Value> {
        match expr {
            Expression::String(s, _) => Ok(Value::String(s.clone())),
            Expression::Number(n, _) => Ok(Value::Number(*n)),
            Expression::Bool(b, _) => Ok(Value::Bool(*b)),
            Expression::Null(_) => Ok(Value::Null),
            
            Expression::Identifier(name, _) => {
                match name.as_str() {
                    "git" => Ok(self.create_git_object()),
                    "env" => Ok(Value::env_object()),
                    "http" => Ok(Value::http_object()),
                    _ => {
                        self.variables.get(name)
                            .cloned()
                            .ok_or_else(|| anyhow::anyhow!("Variable '{}' not found", name))
                    }
                }
            }
            
            Expression::PropertyAccess { chain, span: _ } => {
                self.eval_property_chain(chain)
            }
            
            Expression::MethodCall { receiver, method, args, span: _ } => {
                if let Expression::Identifier(name, _) = receiver.as_ref()
                    && self.builtins.has(name) {
                        let arg_values: Result<Vec<Value>> = args.iter()
                            .map(|a| self.eval_expression(a))
                            .collect();
                        let arg_values = arg_values?;
                        
                        if let Some(result) = self.builtins.call(name, &arg_values)? {
                            return Ok(result);
                        }
                    }
                
                let obj_value = self.eval_expression(receiver)?;
                
                if matches!(method.as_str(), "filter" | "map" | "find" | "any" | "all") && args.len() == 1
                    && let Expression::Closure { param, body, .. } = &args[0] {
                        return self.eval_closure_method(&obj_value, method, param, body);
                    }
                
                let arg_values: Result<Vec<Value>> = args.iter()
                    .map(|a| self.eval_expression(a))
                    .collect();
                obj_value.call_method(method, &arg_values?)
            }
            
            Expression::Binary { left, op, right, span: _ } => {
                let left_val = self.eval_expression(left)?;
                let right_val = self.eval_expression(right)?;
                self.eval_binary_op(&left_val, *op, &right_val)
            }
            
            Expression::Unary { op, expr, span: _ } => {
                let val = self.eval_expression(expr)?;
                self.eval_unary_op(*op, &val)
            }
            
            Expression::Array(elements, _) => {
                let mut values = Vec::with_capacity(elements.len());
                for e in elements {
                    values.push(self.eval_expression(e)?);
                }
                Ok(Value::Array(values))
            }
            
            Expression::InterpolatedString { parts, span: _ } => {
                let mut result = String::new();
                for part in parts {
                    match part {
                        githook_syntax::ast::StringPart::Literal(s) => result.push_str(s),
                        githook_syntax::ast::StringPart::Expression(expr) => {
                            let val = self.eval_expression(expr)?;
                            result.push_str(&val.display());
                        }
                    }
                }
                Ok(Value::String(result))
            }
            
            Expression::Closure { .. } => {
                bail!("Closures cannot be evaluated directly; they must be used as arguments to methods like filter() or map()")
            }
        }
    }
    
    fn eval_property_chain(&self, chain: &[String]) -> Result<Value> {
        if chain.is_empty() {
            bail!("Empty property chain");
        }
        
        let mut current = match chain[0].as_str() {
            "git" => self.create_git_object(),
            "env" => Value::env_object(),
            _ => self.variables.get(&chain[0])
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Variable '{}' not found", chain[0]))?,
        };
        
        for prop in &chain[1..] {
            current = current.get_property(prop)?;
        }
        
        Ok(current)
    }
    
    fn eval_binary_op(&self, left: &Value, op: BinaryOp, right: &Value) -> Result<Value> {
        match op {
            BinaryOp::Eq => Ok(Value::Bool(left.equals(right)?)),
            BinaryOp::Ne => Ok(Value::Bool(left.not_equals(right)?)),
            BinaryOp::Lt => Ok(Value::Bool(left.less_than(right)?)),
            BinaryOp::Le => Ok(Value::Bool(left.less_or_equal(right)?)),
            BinaryOp::Gt => Ok(Value::Bool(left.greater_than(right)?)),
            BinaryOp::Ge => Ok(Value::Bool(left.greater_or_equal(right)?)),
            
            BinaryOp::And => Ok(Value::Bool(left.is_truthy() && right.is_truthy())),
            BinaryOp::Or => Ok(Value::Bool(left.is_truthy() || right.is_truthy())),
            
            BinaryOp::Add => {
                match (left, right) {
                    (Value::String(l), Value::String(r)) => Ok(Value::String(format!("{}{}", l, r))),
                    (Value::String(l), r) => Ok(Value::String(format!("{}{}", l, r.display()))),
                    (l, Value::String(r)) => Ok(Value::String(format!("{}{}", l.display(), r))),
                    (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l + r)),
                    _ => bail!("Cannot add {:?} and {:?}", left, right),
                }
            }
            BinaryOp::Sub => {
                match (left, right) {
                    (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l - r)),
                    _ => bail!("Cannot subtract {:?} from {:?}", right, left),
                }
            }
            BinaryOp::Mul => {
                match (left, right) {
                    (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l * r)),
                    _ => bail!("Cannot multiply {:?} and {:?}", left, right),
                }
            }
            BinaryOp::Div => {
                match (left, right) {
                    (Value::Number(l), Value::Number(r)) => {
                        if *r == 0.0 {
                            bail!("Division by zero");
                        }
                        Ok(Value::Number(l / r))
                    }
                    _ => bail!("Cannot divide {:?} by {:?}", left, right),
                }
            }
            BinaryOp::Mod => {
                match (left, right) {
                    (Value::Number(l), Value::Number(r)) => {
                        if *r == 0.0 {
                            bail!("Modulo by zero");
                        }
                        Ok(Value::Number(l % r))
                    }
                    _ => bail!("Cannot modulo {:?} by {:?}", left, right),
                }
            }
        }
    }
    
    fn eval_unary_op(&self, op: UnaryOp, operand: &Value) -> Result<Value> {
        match op {
            UnaryOp::Not => Ok(Value::Bool(!operand.is_truthy())),
            UnaryOp::Minus => {
                match operand {
                    Value::Number(n) => Ok(Value::Number(-n)),
                    _ => bail!("Cannot negate {:?}", operand),
                }
            }
        }
    }
    
    fn eval_let_value(&self, value: &githook_syntax::ast::LetValue) -> Result<Value> {
        use githook_syntax::ast::LetValue;
        match value {
            LetValue::String(s) => Ok(Value::String(s.clone())),
            LetValue::Number(n) => Ok(Value::Number(*n)),
            LetValue::Array(arr) => {
                let vals: Vec<Value> = arr.iter()
                    .map(|s| Value::String(s.clone()))
                    .collect();
                Ok(Value::Array(vals))
            }
            LetValue::Expression(expr) => self.eval_expression(expr),
        }
    }
    
    fn eval_closure_method(&self, obj: &Value, method: &str, param: &str, body: &Expression) -> Result<Value> {
        match obj {
            Value::Array(arr) => {
                let array_ctx = crate::contexts::ArrayContext::new(arr.clone());
                match method {
                    "filter" => array_ctx.filter(self, param, body),
                    "map" => array_ctx.map(self, param, body),
                    "find" => array_ctx.find(self, param, body),
                    "any" => array_ctx.any(self, param, body),
                    "all" => array_ctx.all(self, param, body),
                    _ => bail!("Unknown closure method: {}", method),
                }
            }
            _ => bail!("Cannot call closure method '{}' on non-array value", method),
        }
    }
   
    pub fn execute_statements(&mut self, statements: &[Statement]) -> Result<ExecutionResult> {
        for stmt in statements {
            let result = self.execute_statement(stmt)?;
            if result.should_stop() || result.is_break() || result.is_continue() {
                return Ok(result);
            }
        }
        Ok(ExecutionResult::Continue)
    }
    
    pub fn execute_statement(&mut self, stmt: &Statement) -> Result<ExecutionResult> {
        match stmt {
            Statement::Run { command, span: _ } => {
                let interpolated = self.interpolate_string(command)?;
                self.run_command(&interpolated)?;
                self.tests_run += 1;
                Ok(ExecutionResult::Continue)
            }
            
            Statement::Print { message, span: _ } => {
                let value = self.eval_expression(message)?;
                let text = match value {
                    Value::String(s) => s,
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    Value::Null => "null".to_string(),
                    Value::Array(arr) => {
                        let strings: Vec<String> = arr.iter().map(|v| match v {
                            Value::String(s) => s.clone(),
                            other => format!("{:?}", other),
                        }).collect();
                        strings.join(", ")
                    },
                    Value::Object(obj) => {
                        if let Some(path_ctx) = &obj.path_context {
                            path_ctx.to_string()
                        } else if let Some(Value::String(s)) = obj.properties.get("name") {
                            s.clone()
                        } else {
                            format!("{}({})", obj.type_name, obj.properties.len())
                        }
                    },
                };
                println!("{}", text);
                Ok(ExecutionResult::Continue)
            }
            
            Statement::Block { message, span: _ } => {
                self.blocks.push(message.clone());
                Ok(ExecutionResult::Blocked)
            }
            
            Statement::Warn { message, span: _ } => {
                let interpolated = self.interpolate_string(message)?;
                self.warnings.push(interpolated);
                Ok(ExecutionResult::Continue)
            }
            
            Statement::Allow { command, span: _ } => {
                if self.verbose {
                    println!("o Explicitly allowed: {}", command);
                }
                Ok(ExecutionResult::Continue)
            }
            
            Statement::Parallel { commands, span: _ } => {
                let interpolated: Result<Vec<String>> = commands.iter()
                    .map(|cmd| self.interpolate_string(cmd))
                    .collect();
                let interpolated = interpolated?;
                
                let results: Vec<Result<()>> = interpolated.par_iter()
                    .map(|cmd| {
                        let output = Command::new("sh")
                            .arg("-c")
                            .arg(cmd)
                            .output()
                            .with_context(|| format!("Failed to execute: {}", cmd))?;
                        
                        if !output.status.success() {
                            bail!("Command failed: {}\nstderr: {}", 
                                  cmd, 
                                  String::from_utf8_lossy(&output.stderr));
                        }
                        Ok(())
                    })
                    .collect();
                
                for result in results {
                    result?;
                }
                self.tests_run += commands.len();
                Ok(ExecutionResult::Continue)
            }
            
            Statement::Let { name, value, span: _ } => {
                let val = self.eval_let_value(value)?;
                self.variables.insert(name.clone(), val);
                Ok(ExecutionResult::Continue)
            }
            
            Statement::Break { span: _ } => {
                Ok(ExecutionResult::Break)
            }
            
            Statement::Continue { span: _ } => {
                Ok(ExecutionResult::ContinueLoop)
            }
            
            Statement::ForEach { collection, var, pattern, body, span: _ } => {
                let coll_value = self.eval_expression(collection)?;
                self.execute_foreach(&coll_value, var, pattern.as_deref(), body)
            }
            
            Statement::If { condition, then_body, else_body, span: _ } => {
                let cond = self.eval_expression(condition)?;
                if cond.is_truthy() {
                    self.execute_statements(then_body)
                } else if let Some(else_stmts) = else_body {
                    self.execute_statements(else_stmts)
                } else {
                    Ok(ExecutionResult::Continue)
                }
            }
            
            Statement::BlockIf { condition, message, interactive, span: _ } => {
                let cond = self.eval_expression(condition)?;
                if cond.is_truthy() {
                    let msg = if let Some(m) = message {
                        self.interpolate_string(m)?
                    } else {
                        "Condition failed".to_string()
                    };
                    self.blocks.push(msg);
                    self.tests_run += 1;
                    let _ = interactive;
                    return Ok(ExecutionResult::Blocked);
                }
                Ok(ExecutionResult::Continue)
            }
            
            Statement::WarnIf { condition, message, interactive, span: _ } => {
                let cond = self.eval_expression(condition)?;
                if cond.is_truthy() {
                    let msg = if let Some(m) = message {
                        self.interpolate_string(m)?
                    } else {
                        "Warning".to_string()
                    };
                    self.warnings.push(msg);
                    self.tests_run += 1;
                    let _ = interactive;
                }
                Ok(ExecutionResult::Continue)
            }
            
            Statement::Match { subject, arms, span: _ } => {
                let subj_value = self.eval_expression(subject)?;
                let arm_tuples: Vec<_> = arms.iter()
                    .map(|arm| (arm.pattern.clone(), arm.body.clone()))
                    .collect();
                self.execute_match(&subj_value, &arm_tuples)
            }
            
            Statement::MacroDef { name, params, body, span: _ } => {
                self.macros.insert(name.clone(), (params.to_vec(), body.clone()));
                Ok(ExecutionResult::Continue)
            }
            
            Statement::MacroCall { namespace, name, args, span: _ } => {
                let (params, body) = if let Some(ns) = namespace {
                    let full_name = format!("{}::{}", ns, name);
                    self.namespaced_macros.get(&full_name)
                        .cloned()
                        .ok_or_else(|| anyhow::anyhow!("Macro '{}::{}' not defined. Did you import the package '@{}/{}'?", ns, name, "preview", ns))?
                } else {
                    self.macros.get(name)
                        .cloned()
                        .ok_or_else(|| anyhow::anyhow!("Macro '{}' not defined", name))?
                };
                
                if params.len() != args.len() {
                    bail!("Macro '{}' expects {} parameters, got {}", name, params.len(), args.len());
                }
                
                let saved_vars = self.variables.clone();
                
                for (param, arg) in params.iter().zip(args.iter()) {
                    let arg_value = self.eval_expression(arg)?;
                    self.variables.insert(param.clone(), arg_value);
                }
                
                let result = self.execute_statements(&body);
                
                for (key, value) in saved_vars {
                    self.variables.entry(key).or_insert(value);
                }
                
                result
            }
            
            Statement::Import { path, alias, span: _ } => {
                let file_path = std::path::Path::new(path);
                
                let import_path = if file_path.is_absolute() {
                    file_path.to_path_buf()
                } else {
                    std::path::PathBuf::from(".githook").join(path)
                };
                
                if !import_path.exists() {
                    bail!("Import file not found: {}", path);
                }
                
                let source = std::fs::read_to_string(&import_path)
                    .with_context(|| format!("Failed to read import file: {}", path))?;
                
                let tokens = githook_syntax::lexer::tokenize(&source)
                    .with_context(|| format!("Failed to tokenize import file: {}", path))?;
                
                let statements = githook_syntax::parser::parse(tokens)
                    .with_context(|| format!("Failed to parse import file: {}", path))?;
                
                if let Some(alias_name) = alias
                    && self.verbose {
                        println!("Importing '{}' as '{}'", path, alias_name);
                    }
                
                self.execute_statements(&statements)
            }
            
            Statement::Use { package, alias, span: _ } => {
                if !package.starts_with('@') {
                    bail!("Package must start with '@', e.g. '@preview/quality'");
                }
                
                let package_path = &package[1..];
                let parts: Vec<&str> = package_path.split('/').collect();
                
                if parts.len() != 2 {
                    bail!("Invalid package format. Expected '@namespace/name', got '{}'", package);
                }
                
                let namespace = parts[0];
                let name = parts[1];
                
                let source = crate::package_resolver::load_package(namespace, name)
                    .with_context(|| format!("Failed to load package: {}", package))?;
                
                let tokens = githook_syntax::lexer::tokenize(&source)
                    .with_context(|| format!("Failed to tokenize package: {}", package))?;
                
                let statements = githook_syntax::parser::parse(tokens)
                    .with_context(|| format!("Failed to parse package: {}", package))?;
                
                let namespace_key = alias.as_ref().unwrap_or(&name.to_string()).clone();
                
                let saved_local_macros = self.macros.clone();
                
                self.execute_statements(&statements)?;
                
                for (macro_name, macro_def) in &self.macros {
                    if !saved_local_macros.contains_key(macro_name) {
                        let full_name = format!("{}::{}", namespace_key, macro_name);
                        self.namespaced_macros.insert(full_name, macro_def.clone());
                    }
                }
                
                self.macros = saved_local_macros;
                
                if self.verbose {
                    println!("Package '{}' loaded into namespace '{}'", package, namespace_key);
                }
                
                Ok(ExecutionResult::Continue)
            }
            
            Statement::Group { name, severity, enabled, body, span: _ } => {
                let sev = severity.as_ref().unwrap_or(&Severity::Critical);
                
                if !enabled {
                    self.add_check(name.clone(), CheckStatus::Skipped, Some("disabled".to_string()), sev.clone());
                    return Ok(ExecutionResult::Continue);
                }
                
                let result = self.execute_statements(body);
                
                match result {
                    Ok(exec_result) => {
                        self.add_check(name.clone(), CheckStatus::Passed, None, sev.clone());
                        Ok(exec_result)
                    }
                    Err(e) => {
                        self.add_check(name.clone(), CheckStatus::Failed, Some(e.to_string()), sev.clone());
                        Err(e)
                    }
                }
            }
            
            Statement::Try { body, catch_var, catch_body, span: _ } => {
                let result = self.execute_statements(body);
                
                match result {
                    Ok(exec_result) => Ok(exec_result),
                    Err(e) => {
                        let var_name = catch_var.as_ref().map(|s| s.as_str()).unwrap_or("error");
                        self.variables.insert(var_name.to_string(), Value::String(e.to_string()));
                        
                        self.execute_statements(catch_body)
                    }
                }
            }
        }
    }
    
    fn execute_foreach(&mut self, collection: &Value, var_name: &str, pattern: Option<&str>, body: &[Statement]) -> Result<ExecutionResult> {
        let items = match collection {
            Value::Array(arr) => arr,
            Value::Object(obj) if obj.type_name == "Git" => {
                if let Some(Value::Array(files)) = obj.get("files") {
                    files
                } else {
                    return Ok(ExecutionResult::Continue);
                }
            }
            _ => bail!("Cannot iterate over {:?}", collection),
        };
        
        if items.is_empty() {
            self.tests_run += 1;
            return Ok(ExecutionResult::Continue);
        }
        
        for item in items {
            if let Some(pattern_str) = pattern {
                let item_name = if let Value::Object(_obj) = item {
                    match item.get_property("name") {
                        Ok(Value::String(name)) => name,
                        _ => continue,
                    }
                } else if let Value::String(s) = item {
                    s.clone()
                } else {
                    continue;
                };
                
                if !Self::matches_pattern(&item_name, pattern_str) {
                    continue;
                }
            }
            
            let old_value = self.variables.insert(var_name.to_string(), item.clone());
            
            let result = self.execute_statements(body)?;
            
            if let Some(old) = old_value {
                self.variables.insert(var_name.to_string(), old);
            } else {
                self.variables.remove(var_name);
            }
            
            if result.is_break() {
                return Ok(ExecutionResult::Continue);
            }
            
            if result.is_continue() {
                continue;
            }
            
            if result.should_stop() {
                return Ok(result);
            }
        }
        
        Ok(ExecutionResult::Continue)
    }
    
    fn execute_match(&mut self, subject: &Value, arms: &[(MatchPattern, Vec<Statement>)]) -> Result<ExecutionResult> {
        for (pattern, body) in arms {
            if self.pattern_matches(pattern, subject)? {
                return self.execute_statements(body);
            }
        }
        
        Ok(ExecutionResult::Continue)
    }
    
    fn pattern_matches(&self, pattern: &MatchPattern, value: &Value) -> Result<bool> {
        match pattern {
            MatchPattern::Expression(expr, _) => {
                let pattern_val = self.eval_expression(expr)?;
                value.equals(&pattern_val)
            }
            
            MatchPattern::Wildcard(s, _) => {
                let value_str = value.as_string()?;
                let pattern_str = s.as_str();
                
                if pattern_str.contains('*') {
                    let regex_pattern = pattern_str
                        .replace(".", "\\.")
                        .replace("*", ".*");
                    let regex = regex::Regex::new(&format!("^{}$", regex_pattern))?;
                    Ok(regex.is_match(&value_str))
                } else {
                    Ok(value_str == pattern_str)
                }
            }
            
            MatchPattern::Underscore(_) => Ok(true),
        }
    }

    fn run_command(&self, cmd: &str) -> Result<()> {
        if self.verbose {
            println!("> Running: {}", cmd);
        }
        
        let output = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output()
            .context(format!("Failed to execute command: {}", cmd))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Command failed: {}\n{}", cmd, stderr);
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.is_empty() {
            print!("{}", stdout);
        }
        
        Ok(())
    }
    
    fn create_git_object(&self) -> Value {
        let git_context = GitContext::new();
        
        let mut git = Object::new("Git")
            .with_git_context(git_context.clone());
        
        let mut files_obj = Object::new("FilesCollection")
            .with_files_context(git_context.files.clone());
        
        let all_files: Vec<Value> = git_context.files.all.iter()
            .map(|path| Value::file_object(path.clone()))
            .collect();
        files_obj.set("all", Value::Array(all_files));
        
        let staged_files: Vec<Value> = git_context.files.staged.iter()
            .map(|path| Value::file_object(path.clone()))
            .collect();
        files_obj.set("staged", Value::Array(staged_files));
        
        let modified_files: Vec<Value> = git_context.files.modified.iter()
            .map(|path| Value::file_object(path.clone()))
            .collect();
        files_obj.set("modified", Value::Array(modified_files));
        
        let added_files: Vec<Value> = git_context.files.added.iter()
            .map(|path| Value::file_object(path.clone()))
            .collect();
        files_obj.set("added", Value::Array(added_files));
        
        let deleted_files: Vec<Value> = git_context.files.deleted.iter()
            .map(|path| Value::file_object(path.clone()))
            .collect();
        files_obj.set("deleted", Value::Array(deleted_files));
        
        let unstaged_files: Vec<Value> = git_context.files.unstaged.iter()
            .map(|path| Value::file_object(path.clone()))
            .collect();
        files_obj.set("unstaged", Value::Array(unstaged_files));
        
        git.set("files", Value::Object(files_obj));
        
        let mut diff_obj = Object::new("DiffCollection");
        
        let added_lines: Vec<Value> = git_context.diff.added_lines.iter()
            .map(|line| Value::String(line.clone()))
            .collect();
        diff_obj.set("added_lines", Value::Array(added_lines));
        
        let removed_lines: Vec<Value> = git_context.diff.removed_lines.iter()
            .map(|line| Value::String(line.clone()))
            .collect();
        diff_obj.set("removed_lines", Value::Array(removed_lines));
        
        git.set("diff", Value::Object(diff_obj));
        
        let mut merge_obj = Object::new("MergeContext");
        
        let merge_source = githook_git::get_merge_source_branch().unwrap_or_else(|_| "unknown".to_string());
        let merge_target = githook_git::get_branch_name().unwrap_or_else(|_| "unknown".to_string());
        
        merge_obj.set("source", Value::String(merge_source));
        merge_obj.set("target", Value::String(merge_target));
        
        git.set("merge", Value::Object(merge_obj));
        
        let branch = Object::new("Branch")
            .with_branch_context(git_context.branch.clone());
        git.set("branch", Value::Object(branch));
        
        let commit_value = if let Some(commit_info) = git_context.commit.clone() {
            let commit = Object::new("Commit")
                .with_commit_context(commit_info);
            Value::Object(commit)
        } else {
            Value::Null
        };
        git.set("commit", commit_value);
        
        let author = Object::new("Author")
            .with_author_context(git_context.author.clone());
        git.set("author", Value::Object(author));
        
        let remote = Object::new("Remote")
            .with_remote_context(git_context.remote.clone());
        git.set("remote", Value::Object(remote));
        
        let stats = Object::new("Stats")
            .with_stats_context(git_context.stats.clone());
        git.set("stats", Value::Object(stats));
        
        Value::Object(git)
    }
    
    fn matches_pattern(text: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        
        if pattern.starts_with('*') && pattern.ends_with('*') {
            let middle = &pattern[1..pattern.len()-1];
            return text.contains(middle);
        }
        
        if let Some(suffix) = pattern.strip_prefix('*') {
            return text.ends_with(suffix);
        }
        
        if let Some(prefix) = pattern.strip_suffix('*') {
            return text.starts_with(prefix);
        }
        
        text == pattern
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use githook_syntax::{lexer, parser};
    use githook_syntax::error::Span;

    fn dummy_span() -> Span {
        Span::new(1, 1, 0, 0)
    }

    fn parse_and_execute(input: &str) -> Result<Executor> {
        let tokens = lexer::tokenize(input)?;
        let ast = parser::parse(tokens)?;
        let mut executor = Executor::new();
        for stmt in ast {
            executor.execute_statement(&stmt)?;
        }
        Ok(executor)
    }

    #[test]
    fn test_execute_let_statement() {
        let input = r#"let x = 42"#;
        let executor = parse_and_execute(input).unwrap();
        
        let val = executor.variables.get("x").unwrap();
        assert!(matches!(val, Value::Number(42.0)));
    }

    #[test]
    fn test_execute_let_string() {
        let input = r#"let name = "Alice""#;
        let executor = parse_and_execute(input).unwrap();
        
        let val = executor.variables.get("name").unwrap();
        if let Value::String(s) = val {
            assert_eq!(s, "Alice");
        } else {
            panic!("Expected string value");
        }
    }

    #[test]
    fn test_execute_multiple_lets() {
        let input = r#"
            let x = 1
            let y = 2
            let z = 3
        "#;
        let executor = parse_and_execute(input).unwrap();
        
        assert!(executor.variables.contains_key("x"));
        assert!(executor.variables.contains_key("y"));
        assert!(executor.variables.contains_key("z"));
    }

    #[test]
    fn test_eval_binary_expression() {
        let input = r#"let result = 5 + 3"#;
        let executor = parse_and_execute(input).unwrap();
        
        let val = executor.variables.get("result").unwrap();
        assert!(matches!(val, Value::Number(8.0)));
    }

    #[test]
    fn test_eval_comparison() {
        let executor = Executor::new();
        
        let left = Expression::Number(5.0, dummy_span());
        let right = Expression::Number(3.0, dummy_span());
        let expr = Expression::Binary {
            left: Box::new(left),
            op: BinaryOp::Gt,
            right: Box::new(right),
            span: dummy_span(),
        };
        
        let result = executor.eval_expression(&expr).unwrap();
        assert!(matches!(result, Value::Bool(true)));
    }

    #[test]
    fn test_eval_logical_and() {
        let executor = Executor::new();
        
        let left = Expression::Bool(true, dummy_span());
        let right = Expression::Bool(false, dummy_span());
        let expr = Expression::Binary {
            left: Box::new(left),
            op: BinaryOp::And,
            right: Box::new(right),
            span: dummy_span(),
        };
        
        let result = executor.eval_expression(&expr).unwrap();
        assert!(matches!(result, Value::Bool(false)));
    }

    #[test]
    fn test_eval_logical_or() {
        let executor = Executor::new();
        
        let left = Expression::Bool(true, dummy_span());
        let right = Expression::Bool(false, dummy_span());
        let expr = Expression::Binary {
            left: Box::new(left),
            op: BinaryOp::Or,
            right: Box::new(right),
            span: dummy_span(),
        };
        
        let result = executor.eval_expression(&expr).unwrap();
        assert!(matches!(result, Value::Bool(true)));
    }

    #[test]
    fn test_eval_string_literal() {
        let executor = Executor::new();
        let expr = Expression::String("hello".to_string(), dummy_span());
        let result = executor.eval_expression(&expr).unwrap();
        
        assert!(matches!(result, Value::String(s) if s == "hello"));
    }

    #[test]
    fn test_eval_number_literal() {
        let executor = Executor::new();
        let expr = Expression::Number(3.14, dummy_span());
        let result = executor.eval_expression(&expr).unwrap();
        
        assert!(matches!(result, Value::Number(n) if (n - 3.14).abs() < 0.001));
    }

    #[test]
    fn test_eval_bool_literal() {
        let executor = Executor::new();
        let expr = Expression::Bool(true, dummy_span());
        let result = executor.eval_expression(&expr).unwrap();
        
        assert!(matches!(result, Value::Bool(true)));
    }

    #[test]
    fn test_eval_null_literal() {
        let executor = Executor::new();
        let expr = Expression::Null(dummy_span());
        let result = executor.eval_expression(&expr).unwrap();
        
        assert!(matches!(result, Value::Null));
    }

    #[test]
    fn test_variable_not_found() {
        let executor = Executor::new();
        let expr = Expression::Identifier("unknown".to_string(), dummy_span());
        let result = executor.eval_expression(&expr);
        
        assert!(result.is_err());
    }

    #[test]
    fn test_set_and_get_variable() {
        let mut executor = Executor::new();
        executor.set_variable("test".to_string(), Value::Number(100.0));
        
        assert!(executor.variables.contains_key("test"));
        assert!(matches!(executor.variables.get("test"), Some(Value::Number(100.0))));
    }

    #[test]
    fn test_matches_pattern() {
        assert!(Executor::matches_pattern("test.rs", "*.rs"));
        assert!(Executor::matches_pattern("test.rs", "test.*"));
        assert!(Executor::matches_pattern("test.rs", "*est*"));
        assert!(Executor::matches_pattern("anything", "*"));
        assert!(!Executor::matches_pattern("test.py", "*.rs"));
    }
}
