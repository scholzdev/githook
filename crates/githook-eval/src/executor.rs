use anyhow::{Result, bail, Context as _};
use rustc_hash::FxHashMap;  // Faster than ahash for small string keys
use std::process::Command;
use rayon::prelude::*;

use crate::value::{Value, Object};
use crate::contexts::GitContext;
use crate::builtins::BuiltinRegistry;
use githook_syntax::ast::{Statement, Expression, BinaryOp, UnaryOp, MatchPattern, Severity};

// Use FxHashMap for string-keyed maps (identifiers, variable names)
type VariableMap = FxHashMap<String, Value>;
type MacroMap = FxHashMap<String, (Vec<String>, Vec<Statement>)>;

// ============================================================================
// CHECK TRACKING
// ============================================================================

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

// ============================================================================
// EXECUTOR V2 - Expression evaluation and statement execution
// ============================================================================

#[derive(Clone)]
pub struct Executor {
    /// Global variables (let bindings) - using FxHashMap for identifier lookups
    pub variables: VariableMap,
    
    /// Git context (injected)
    git_files: Vec<String>,
    
    /// Execution mode
    pub verbose: bool,
    
    /// Collected warnings
    pub warnings: Vec<String>,
    
    /// Collected blocks
    pub blocks: Vec<String>,
    
    /// Test counter
    pub tests_run: usize,
    
    /// Macro definitions (name -> (params, body)) - FxHashMap for macro name lookups
    macros: MacroMap,
    
    /// Namespaced macro definitions (namespace::name -> (params, body))
    namespaced_macros: MacroMap,
    
    /// Check results for structured output
    pub check_results: Vec<CheckResult>,
    
    /// Built-in function registry
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
    
    /// Add a check result for structured output
    pub fn add_check(&mut self, name: String, status: CheckStatus, reason: Option<String>, severity: Severity) {
        self.check_results.push(CheckResult { name, status, reason, severity });
    }
    
    // ========================================================================
    // EXPRESSION EVALUATION
    // ========================================================================
    
    pub fn eval_expression(&self, expr: &Expression) -> Result<Value> {
        match expr {
            Expression::String(s, _) => Ok(Value::String(s.clone())),
            Expression::Number(n, _) => Ok(Value::Number(*n)),
            Expression::Bool(b, _) => Ok(Value::Bool(*b)),
            Expression::Null(_) => Ok(Value::Null),
            
            Expression::Identifier(name, _) => {
                // Built-in objects
                match name.as_str() {
                    "git" => Ok(self.create_git_object()),
                    "env" => Ok(Value::env_object()),
                    _ => {
                        // User variables
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
                // Check if this is a built-in function call (e.g., file("path"))
                if let Expression::Identifier(name, _) = receiver.as_ref() {
                    if self.builtins.has(name) {
                        // Evaluate arguments
                        let arg_values: Result<Vec<Value>> = args.iter()
                            .map(|a| self.eval_expression(a))
                            .collect();
                        let arg_values = arg_values?;
                        
                        // Call built-in function
                        if let Some(result) = self.builtins.call(name, &arg_values)? {
                            return Ok(result);
                        }
                    }
                }
                
                let obj_value = self.eval_expression(receiver)?;
                
                // Special handling for methods that accept closures
                if matches!(method.as_str(), "filter" | "map" | "find" | "any" | "all") && args.len() == 1
                    && let Expression::Closure { param, body, .. } = &args[0] {
                        return self.eval_closure_method(&obj_value, method, param, body);
                    }
                
                // Regular method call with evaluated arguments
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
                // Pre-allocate capacity for better performance
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
        
        // First element: resolve to value
        let mut current = match chain[0].as_str() {
            "git" => self.create_git_object(),
            "env" => Value::env_object(),
            _ => self.variables.get(&chain[0])
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Variable '{}' not found", chain[0]))?,
        };
        
        // Rest: property access
        for prop in &chain[1..] {
            current = current.get_property(prop)?;
        }
        
        Ok(current)
    }
    
    fn eval_binary_op(&self, left: &Value, op: BinaryOp, right: &Value) -> Result<Value> {
        match op {
            // Comparison
            BinaryOp::Eq => Ok(Value::Bool(left.equals(right)?)),
            BinaryOp::Ne => Ok(Value::Bool(left.not_equals(right)?)),
            BinaryOp::Lt => Ok(Value::Bool(left.less_than(right)?)),
            BinaryOp::Le => Ok(Value::Bool(left.less_or_equal(right)?)),
            BinaryOp::Gt => Ok(Value::Bool(left.greater_than(right)?)),
            BinaryOp::Ge => Ok(Value::Bool(left.greater_or_equal(right)?)),
            
            // Logical
            BinaryOp::And => Ok(Value::Bool(left.is_truthy() && right.is_truthy())),
            BinaryOp::Or => Ok(Value::Bool(left.is_truthy() || right.is_truthy())),
            
            // Arithmetic
            BinaryOp::Add => {
                match (left, right) {
                    // String concatenation
                    (Value::String(l), Value::String(r)) => Ok(Value::String(format!("{}{}", l, r))),
                    (Value::String(l), r) => Ok(Value::String(format!("{}{}", l, r.display()))),
                    (l, Value::String(r)) => Ok(Value::String(format!("{}{}", l.display(), r))),
                    // Number addition
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
                // Create ArrayContext to delegate to
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
    
    // ========================================================================
    // STATEMENT EXECUTION
    // ========================================================================
    
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
                // Evaluate the expression and convert to string
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
                        // Try to get a reasonable string representation
                        if let Some(path_ctx) = &obj.path_context {
                            path_ctx.to_string()
                        } else if let Some(name_prop) = obj.properties.get("name") {
                            if let Value::String(s) = name_prop {
                                s.clone()
                            } else {
                                format!("{}({})", obj.type_name, obj.properties.len())
                            }
                        } else {
                            format!("{}({})", obj.type_name, obj.properties.len())
                        }
                    },
                };
                println!("{}", text);
                Ok(ExecutionResult::Continue)
            }
            
            Statement::Block { message, span: _ } => {
                // Block means "stop and show message" - like a hard block
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
                // Parallel execution using rayon - interpolate strings first
                let interpolated: Result<Vec<String>> = commands.iter()
                    .map(|cmd| self.interpolate_string(cmd))
                    .collect();
                let interpolated = interpolated?;
                
                // Run commands in parallel
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
                
                // Check for errors
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
                    if interactive.is_some() {
                        // TODO: Interactive prompts
                    }
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
                    if interactive.is_some() {
                        // TODO: Interactive prompts
                    }
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
                // Store macro definition (convert SmallVec to Vec)
                self.macros.insert(name.clone(), (params.to_vec(), body.clone()));
                Ok(ExecutionResult::Continue)
            }
            
            Statement::MacroCall { namespace, name, args, span: _ } => {
                // Lookup macro
                let (params, body) = if let Some(ns) = namespace {
                    // Namespaced macro call: @namespace:macro_name
                    let full_name = format!("{}::{}", ns, name);
                    self.namespaced_macros.get(&full_name)
                        .cloned()
                        .ok_or_else(|| anyhow::anyhow!("Macro '{}::{}' not defined. Did you import the package '@{}/{}'?", ns, name, "preview", ns))?
                } else {
                    // Local macro call: @macro_name
                    self.macros.get(name)
                        .cloned()
                        .ok_or_else(|| anyhow::anyhow!("Macro '{}' not defined", name))?
                };
                
                // Check parameter count
                if params.len() != args.len() {
                    bail!("Macro '{}' expects {} parameters, got {}", name, params.len(), args.len());
                }
                
                // Save current variables
                let saved_vars = self.variables.clone();
                
                // Bind parameters
                for (param, arg) in params.iter().zip(args.iter()) {
                    let arg_value = self.eval_expression(arg)?;
                    self.variables.insert(param.clone(), arg_value);
                }
                
                // Execute macro body
                let result = self.execute_statements(&body);
                
                // Restore variables (but keep any new bindings from macro)
                for (key, value) in saved_vars {
                    self.variables.entry(key).or_insert(value);
                }
                
                result
            }
            
            Statement::Import { path, alias, span: _ } => {
                // Import local .ghook file
                let file_path = std::path::Path::new(path);
                
                // Resolve relative to current file or workspace
                let import_path = if file_path.is_absolute() {
                    file_path.to_path_buf()
                } else {
                    // Assume relative to .githook directory for now
                    std::path::PathBuf::from(".githook").join(path)
                };
                
                if !import_path.exists() {
                    bail!("Import file not found: {}", path);
                }
                
                // Read and parse imported file
                let source = std::fs::read_to_string(&import_path)
                    .with_context(|| format!("Failed to read import file: {}", path))?;
                
                let tokens = githook_syntax::lexer::tokenize(&source)
                    .with_context(|| format!("Failed to tokenize import file: {}", path))?;
                
                let statements = githook_syntax::parser::parse(tokens)
                    .with_context(|| format!("Failed to parse import file: {}", path))?;
                
                // Execute imported statements
                if let Some(alias_name) = alias {
                    // TODO: Support namespaced imports with alias
                    // For now just execute directly
                    if self.verbose {
                        println!("Importing '{}' as '{}'", path, alias_name);
                    }
                }
                
                self.execute_statements(&statements)
            }
            
            Statement::Use { package, alias, span: _ } => {
                // Use external package: use "@namespace/name"
                if !package.starts_with('@') {
                    bail!("Package must start with '@', e.g. '@preview/quality'");
                }
                
                let package_path = &package[1..]; // Remove @
                let parts: Vec<&str> = package_path.split('/').collect();
                
                if parts.len() != 2 {
                    bail!("Invalid package format. Expected '@namespace/name', got '{}'", package);
                }
                
                let namespace = parts[0];
                let name = parts[1];
                
                // Load package (auto-fetches if not cached)
                let source = crate::package_resolver::load_package(namespace, name)
                    .with_context(|| format!("Failed to load package: {}", package))?;
                
                let tokens = githook_syntax::lexer::tokenize(&source)
                    .with_context(|| format!("Failed to tokenize package: {}", package))?;
                
                let statements = githook_syntax::parser::parse(tokens)
                    .with_context(|| format!("Failed to parse package: {}", package))?;
                
                // Execute package statements and register macros in namespace
                let namespace_key = alias.as_ref().unwrap_or(&name.to_string()).clone();
                
                // Save current macros
                let saved_local_macros = self.macros.clone();
                
                // Execute package (macros will be added to self.macros)
                self.execute_statements(&statements)?;
                
                // Move all newly defined macros to namespaced_macros
                for (macro_name, macro_def) in &self.macros {
                    if !saved_local_macros.contains_key(macro_name) {
                        let full_name = format!("{}::{}", namespace_key, macro_name);
                        self.namespaced_macros.insert(full_name, macro_def.clone());
                    }
                }
                
                // Restore local macros (remove package macros from local scope)
                self.macros = saved_local_macros;
                
                if self.verbose {
                    println!("✓ Package '{}' loaded into namespace '{}'", package, namespace_key);
                }
                
                Ok(ExecutionResult::Continue)
            }
            
            Statement::Group { name, severity, enabled, body, span: _ } => {
                let sev = severity.as_ref().unwrap_or(&Severity::Critical);
                
                if !enabled {
                    self.add_check(name.clone(), CheckStatus::Skipped, Some("disabled".to_string()), sev.clone());
                    return Ok(ExecutionResult::Continue);
                }
                
                // Try to execute the group body
                let result = self.execute_statements(body);
                
                match result {
                    Ok(exec_result) => {
                        // Check if the group had any actual work (look for tests_run before/after)
                        // For now, mark as Passed
                        self.add_check(name.clone(), CheckStatus::Passed, None, sev.clone());
                        Ok(exec_result)
                    }
                    Err(e) => {
                        // Group failed
                        self.add_check(name.clone(), CheckStatus::Failed, Some(e.to_string()), sev.clone());
                        Err(e)
                    }
                }
            }
            
            Statement::Try { body, catch_var, catch_body, span: _ } => {
                // Execute try body
                let result = self.execute_statements(body);
                
                match result {
                    Ok(exec_result) => Ok(exec_result),
                    Err(e) => {
                        // Error occurred, execute catch body
                        // Bind error message to variable (use explicit name or default to "error")
                        let var_name = catch_var.as_ref().map(|s| s.as_str()).unwrap_or("error");
                        self.variables.insert(var_name.to_string(), Value::String(e.to_string()));
                        
                        // Execute catch body
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
                // git.all_files or git.files.staged
                if let Some(Value::Array(files)) = obj.get("files") {
                    files
                } else {
                    return Ok(ExecutionResult::Continue);
                }
            }
            _ => bail!("Cannot iterate over {:?}", collection),
        };
        
        // Track if collection was empty
        if items.is_empty() {
            self.tests_run += 1; // Count foreach as 1 check even if empty
            return Ok(ExecutionResult::Continue);
        }
        
        for item in items {
            // Apply pattern filter if specified
            if let Some(pattern_str) = pattern {
                // Get the file name from the item
                let item_name = if let Value::Object(_obj) = item {
                    // Use get_property() to access File context properties
                    match item.get_property("name") {
                        Ok(Value::String(name)) => name,
                        _ => continue, // Skip if no name property or not a string
                    }
                } else if let Value::String(s) = item {
                    s.clone()
                } else {
                    continue; // Skip if not an object or string
                };
                
                // Check if item matches pattern (wildcard matching)
                if !Self::matches_pattern(&item_name, pattern_str) {
                    continue; // Skip this item
                }
            }
            
            // Set loop variable
            let old_value = self.variables.insert(var_name.to_string(), item.clone());
            
            // Execute body - this will increment tests_run for each check
            let result = self.execute_statements(body)?;
            
            // Restore old value
            if let Some(old) = old_value {
                self.variables.insert(var_name.to_string(), old);
            } else {
                self.variables.remove(var_name);
            }
            
            // Handle break: exit the loop
            if result.is_break() {
                return Ok(ExecutionResult::Continue);
            }
            
            // Handle continue: skip to next iteration
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
        
        // No match found
        Ok(ExecutionResult::Continue)
    }
    
    fn pattern_matches(&self, pattern: &MatchPattern, value: &Value) -> Result<bool> {
        match pattern {
            MatchPattern::Expression(expr, _) => {
                let pattern_val = self.eval_expression(expr)?;
                value.equals(&pattern_val)
            }
            
            MatchPattern::Wildcard(s, _) => {
                // Simple glob matching
                let value_str = value.as_string()?;
                let pattern_str = s.as_str();
                
                if pattern_str.contains('*') {
                    // Convert glob to regex
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
    
    // ========================================================================
    // HELPERS
    // ========================================================================
    
    fn interpolate_string(&self, s: &str) -> Result<String> {
        let mut result = String::new();
        let mut chars = s.chars().peekable();
        
        while let Some(ch) = chars.next() {
            if ch == '$' && chars.peek() == Some(&'{') {
                chars.next(); // consume '{'
                
                // Extract expression until '}'
                let mut expr_str = String::new();
                let mut depth = 1;
                for ch in chars.by_ref() {
                    if ch == '{' {
                        depth += 1;
                        expr_str.push(ch);
                    } else if ch == '}' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                        expr_str.push(ch);
                    } else {
                        expr_str.push(ch);
                    }
                }
                
                // Parse the expression properly using the parser
                use githook_syntax::lexer::tokenize;
                use githook_syntax::parser::Parser;
                
                let tokens = tokenize(&expr_str)?;
                let mut expr_parser = Parser::new(tokens);
                let expr = expr_parser.parse_expression()?;
                
                let value = self.eval_expression(&expr)?;
                result.push_str(&value.display());
            } else {
                result.push(ch);
            }
        }
        
        Ok(result)
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
        
        // Always print stdout from commands
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.is_empty() {
            print!("{}", stdout);
        }
        
        Ok(())
    }
    
    fn create_git_object(&self) -> Value {
        // Get git context from githook-git crate
        let git_context = GitContext::new();
        
        // Create Git object with context
        let mut git = Object::new("Git")
            .with_git_context(git_context.clone());
        
        // files object with staged and all arrays
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
        
        // diff object with added/removed lines
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
        
        // merge object with source and target
        let mut merge_obj = Object::new("MergeContext");
        
        let merge_source = githook_git::get_merge_source_branch().unwrap_or_else(|_| "unknown".to_string());
        let merge_target = githook_git::get_branch_name().unwrap_or_else(|_| "unknown".to_string());
        
        merge_obj.set("source", Value::String(merge_source));
        merge_obj.set("target", Value::String(merge_target));
        
        git.set("merge", Value::Object(merge_obj));
        
        // branch object with context
        let branch = Object::new("Branch")
            .with_branch_context(git_context.branch.clone());
        git.set("branch", Value::Object(branch));
        
        // commit object (null if no commit exists)
        let commit_value = if let Some(commit_info) = git_context.commit.clone() {
            let commit = Object::new("Commit")
                .with_commit_context(commit_info);
            Value::Object(commit)
        } else {
            Value::Null
        };
        git.set("commit", commit_value);
        
        // author object with context
        let author = Object::new("Author")
            .with_author_context(git_context.author.clone());
        git.set("author", Value::Object(author));
        
        // remote object with context
        let remote = Object::new("Remote")
            .with_remote_context(git_context.remote.clone());
        git.set("remote", Value::Object(remote));
        
        // stats object with context
        let stats = Object::new("Stats")
            .with_stats_context(git_context.stats.clone());
        git.set("stats", Value::Object(stats));
        
        Value::Object(git)
    }
    
    /// Simple wildcard pattern matching
    /// Supports: *.ext, prefix*, *suffix, *middle*, exact_match
    fn matches_pattern(text: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true; // Match everything
        }
        
        if pattern.starts_with('*') && pattern.ends_with('*') {
            // *middle* - contains
            let middle = &pattern[1..pattern.len()-1];
            return text.contains(middle);
        }
        
        if let Some(suffix) = pattern.strip_prefix('*') {
            // *suffix - ends with
            return text.ends_with(suffix);
        }
        
        if let Some(prefix) = pattern.strip_suffix('*') {
            // prefix* - starts with
            return text.starts_with(prefix);
        }
        
        // Exact match
        text == pattern
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// EXECUTION RESULT
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionResult {
    /// Continue execution
    Continue,
    
    /// Block execution (block if triggered)
    Blocked,
    
    /// Break from loop
    Break,
    
    /// Continue to next loop iteration
    ContinueLoop,
}

impl ExecutionResult {
    pub fn should_stop(&self) -> bool {
        matches!(self, ExecutionResult::Blocked)
    }
    
    pub fn is_break(&self) -> bool {
        matches!(self, ExecutionResult::Break)
    }
    
    pub fn is_continue(&self) -> bool {
        matches!(self, ExecutionResult::ContinueLoop)
    }
}
