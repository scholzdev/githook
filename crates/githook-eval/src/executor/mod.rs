//! Tree-walking interpreter for Githook scripts.
//!
//! Submodules:
//! - [`expressions`] – expression evaluation and operator dispatch
//! - [`git_objects`] – builds the `git` runtime object tree

mod expressions;
mod git_objects;

use anyhow::{Context as _, Result};
use rayon::prelude::*;
use rustc_hash::FxHashMap;
use std::process::Command;

use crate::bail_span;
use crate::builtins::BuiltinRegistry;
use crate::config::Config;
use crate::control_flow::ExecutionResult;
use crate::value::Value;
use githook_syntax::ast::{MatchPattern, Severity, Statement};
use githook_syntax::error::Span;

type VariableMap = FxHashMap<String, Value>;
type MacroMap = FxHashMap<String, (Vec<String>, Vec<Statement>)>;

/// The outcome of a single named check within a `group`.
#[derive(Debug, Clone, PartialEq)]
pub enum CheckStatus {
    /// The check passed.
    Passed,
    /// The check was skipped (e.g. group was disabled).
    Skipped,
    /// The check failed.
    Failed,
}

/// The result of a named check, including its severity and optional reason.
#[derive(Debug, Clone)]
pub struct CheckResult {
    /// The display name of the check.
    pub name: String,
    /// Whether the check passed, failed, or was skipped.
    pub status: CheckStatus,
    /// An optional human-readable reason (e.g. the block message).
    pub reason: Option<String>,
    /// The severity level of this check.
    pub severity: Severity,
}

/// The tree-walking interpreter for Githook scripts.
///
/// Create an `Executor`, optionally configure it with [`with_git_files`](`Executor::with_git_files`),
/// then call [`execute_statement`](`Executor::execute_statement`) for each parsed [`Statement`].
#[derive(Clone)]
pub struct Executor {
    /// User-defined and built-in variables.
    pub variables: VariableMap,
    git_files: Vec<String>,
    /// Enable verbose output.
    pub verbose: bool,
    /// Warnings collected during execution.
    pub warnings: Vec<String>,
    /// Block messages collected during execution.
    pub blocks: Vec<String>,
    /// Number of tests (run commands) executed.
    pub tests_run: usize,
    macros: MacroMap,
    namespaced_macros: MacroMap,
    /// Results from `group` checks.
    pub check_results: Vec<CheckResult>,
    builtins: BuiltinRegistry,
    /// Runtime configuration.
    pub config: Config,
}

impl Executor {
    /// Creates a new executor with default settings and an empty variable scope.
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
            config: Config::default(),
        }
    }

    /// Builder: pre-loads staged git files for the `git.files.*` context.
    pub fn with_git_files(mut self, files: Vec<String>) -> Self {
        self.git_files = files;
        self
    }

    /// Builder: sets the runtime configuration.
    pub fn with_config(mut self, config: Config) -> Self {
        self.config = config;
        self
    }

    /// Inserts a variable into the executor's scope.
    pub fn set_variable(&mut self, name: String, value: Value) {
        self.variables.insert(name, value);
    }

    /// Records a check result (used by `group` execution).
    pub fn add_check(
        &mut self,
        name: String,
        status: CheckStatus,
        reason: Option<String>,
        severity: Severity,
    ) {
        self.check_results.push(CheckResult {
            name,
            status,
            reason,
            severity,
        });
    }

    /// Executes a slice of statements in order, stopping early on block/break/continue.
    pub fn execute_statements(&mut self, statements: &[Statement]) -> Result<ExecutionResult> {
        for stmt in statements {
            let result = self.execute_statement(stmt)?;
            if result.should_stop() || result.is_break() || result.is_continue() {
                return Ok(result);
            }
        }
        Ok(ExecutionResult::Continue)
    }

    /// Executes a single statement and returns the control-flow result.
    pub fn execute_statement(&mut self, stmt: &Statement) -> Result<ExecutionResult> {
        match stmt {
            Statement::Run { command, span } => {
                let value = self.eval_expression(command)?;
                let cmd_str = match &value {
                    Value::String(s) => s.clone(),
                    other => bail_span!(span, "'run' expects a string command, got {:?}", other),
                };
                self.run_command(&cmd_str, span)?;
                self.tests_run += 1;
                Ok(ExecutionResult::Continue)
            }

            Statement::Print { message, span: _ } => {
                let value = self.eval_expression(message)?;
                println!("{}", value.display());
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

            Statement::Parallel { commands, span } => {
                let interpolated: Result<Vec<String>> = commands
                    .iter()
                    .map(|cmd| self.interpolate_string(cmd))
                    .collect();
                let interpolated = interpolated?;
                let span = *span;
                let verbose = self.verbose;
                let timeout = self.config.command_timeout;

                // If max_parallel_threads > 0, use a custom thread pool
                let results: Vec<Result<std::process::Output>> = if self.config.max_parallel_threads > 0 {
                    let pool = rayon::ThreadPoolBuilder::new()
                        .num_threads(self.config.max_parallel_threads)
                        .build()
                        .context("Failed to create thread pool")?;
                    pool.install(|| {
                        interpolated
                            .par_iter()
                            .map(|cmd| Self::run_parallel_command(cmd, span, verbose, timeout))
                            .collect()
                    })
                } else {
                    interpolated
                        .par_iter()
                        .map(|cmd| Self::run_parallel_command(cmd, span, verbose, timeout))
                        .collect()
                };

                for result in results {
                    let output = result?;
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if !stdout.is_empty() {
                        print!("{}", stdout);
                    }
                }
                self.tests_run += commands.len();
                Ok(ExecutionResult::Continue)
            }

            Statement::Let {
                name,
                value,
                span: _,
            } => {
                let val = self.eval_let_value(value)?;
                self.variables.insert(name.clone(), val);
                Ok(ExecutionResult::Continue)
            }

            Statement::Break { span: _ } => Ok(ExecutionResult::Break),

            Statement::Continue { span: _ } => Ok(ExecutionResult::ContinueLoop),

            Statement::ForEach {
                collection,
                var,
                pattern,
                body,
                span,
            } => {
                let coll_value = self.eval_expression(collection)?;
                self.execute_foreach(&coll_value, var, pattern.as_deref(), body, span)
            }

            Statement::If {
                condition,
                then_body,
                else_body,
                span: _,
            } => {
                let cond = self.eval_expression(condition)?;
                if cond.is_truthy() {
                    self.execute_statements(then_body)
                } else if let Some(else_stmts) = else_body {
                    self.execute_statements(else_stmts)
                } else {
                    Ok(ExecutionResult::Continue)
                }
            }

            Statement::BlockIf {
                condition,
                message,
                interactive,
                span: _,
            } => {
                let cond = self.eval_expression(condition)?;
                if cond.is_truthy() {
                    let msg = if let Some(m) = message {
                        self.interpolate_string(m)?
                    } else {
                        "Condition failed".to_string()
                    };
                    self.blocks.push(msg);
                    self.tests_run += 1;
                    // TODO: `interactive` is a reserved keyword for future use.
                    // When set, the executor should prompt the user instead of
                    // blocking unconditionally (e.g. "Allow anyway? [y/N]").
                    let _ = interactive;
                    return Ok(ExecutionResult::Blocked);
                }
                Ok(ExecutionResult::Continue)
            }

            Statement::WarnIf {
                condition,
                message,
                interactive,
                span: _,
            } => {
                let cond = self.eval_expression(condition)?;
                if cond.is_truthy() {
                    let msg = if let Some(m) = message {
                        self.interpolate_string(m)?
                    } else {
                        "Warning".to_string()
                    };
                    self.warnings.push(msg);
                    self.tests_run += 1;
                    // TODO: `interactive` is a reserved keyword for future use.
                    // When set, the executor should prompt the user before
                    // continuing (e.g. "Acknowledge warning? [y/N]").
                    let _ = interactive;
                }
                Ok(ExecutionResult::Continue)
            }

            Statement::Match {
                subject,
                arms,
                span: _,
            } => {
                let subj_value = self.eval_expression(subject)?;
                let arm_tuples: Vec<_> = arms
                    .iter()
                    .map(|arm| (arm.pattern.clone(), arm.body.clone()))
                    .collect();
                self.execute_match(&subj_value, &arm_tuples)
            }

            Statement::MacroDef {
                name,
                params,
                body,
                span: _,
            } => {
                self.macros
                    .insert(name.clone(), (params.to_vec(), body.clone()));
                Ok(ExecutionResult::Continue)
            }

            Statement::MacroCall {
                namespace,
                name,
                args,
                span,
            } => {
                let (params, body) = if let Some(ns) = namespace {
                    let full_name = format!("{}::{}", ns, name);
                    self.namespaced_macros
                        .get(&full_name)
                        .cloned()
                        .ok_or_else(|| {
                            anyhow::anyhow!(crate::error::EvalError::spanned(
                                format!(
                                    "Macro '{}::{}' not defined. Did you import the package '@{}/{}'?",
                                    ns, name, "preview", ns
                                ),
                                span,
                            ))
                        })?
                } else {
                    self.macros.get(name).cloned().ok_or_else(|| {
                        anyhow::anyhow!(crate::error::EvalError::spanned(
                            format!("Macro '{}' not defined", name),
                            span,
                        ))
                    })?
                };

                if params.len() != args.len() {
                    bail_span!(
                        span,
                        "Macro '{}' expects {} parameters, got {}",
                        name,
                        params.len(),
                        args.len()
                    );
                }

                let saved_vars = self.variables.clone();

                for (param, arg) in params.iter().zip(args.iter()) {
                    let arg_value = self.eval_expression(arg)?;
                    self.variables.insert(param.clone(), arg_value);
                }

                let result = self.execute_statements(&body);

                // Restore macro parameters to their pre-call values (or remove
                // them if they didn't exist before).  Other variables that the
                // macro body modified are intentionally kept — macros act as
                // inline expansions, not isolated scopes.
                for param in &params {
                    if let Some(original) = saved_vars.get(param) {
                        self.variables.insert(param.clone(), original.clone());
                    } else {
                        self.variables.remove(param);
                    }
                }

                result
            }

            Statement::Import { path, alias, span } => {
                let file_path = std::path::Path::new(path);

                let import_path = if file_path.is_absolute() {
                    file_path.to_path_buf()
                } else {
                    std::path::PathBuf::from(".githook").join(path)
                };

                if !import_path.exists() {
                    bail_span!(span, "Import file not found: {}", path);
                }

                let source = std::fs::read_to_string(&import_path)
                    .with_context(|| format!("Failed to read import file: {}", path))?;

                let tokens = githook_syntax::lexer::tokenize(&source)
                    .with_context(|| format!("Failed to tokenize import file: {}", path))?;

                let statements = githook_syntax::parser::parse(tokens)
                    .with_context(|| format!("Failed to parse import file: {}", path))?;

                if let Some(alias_name) = alias {
                    if self.verbose {
                        println!("Importing '{}' as '{}'", path, alias_name);
                    }

                    // Namespace the imported macros under the alias, just
                    // like `use` does for packages.
                    let saved_local_macros = self.macros.clone();

                    self.execute_statements(&statements)?;

                    for (macro_name, macro_def) in &self.macros {
                        if !saved_local_macros.contains_key(macro_name) {
                            let full_name = format!("{}::{}", alias_name, macro_name);
                            self.namespaced_macros.insert(full_name, macro_def.clone());
                        }
                    }

                    self.macros = saved_local_macros;

                    Ok(ExecutionResult::Continue)
                } else {
                    // No alias — macros become globally available.
                    self.execute_statements(&statements)
                }
            }

            Statement::Use {
                package,
                alias,
                span,
            } => {
                if !package.starts_with('@') {
                    bail_span!(span, "Package must start with '@', e.g. '@preview/quality'");
                }

                let package_path = &package[1..];
                let parts: Vec<&str> = package_path.split('/').collect();

                if parts.len() != 2 {
                    bail_span!(
                        span,
                        "Invalid package format. Expected '@namespace/name', got '{}'",
                        package
                    );
                }

                let namespace = parts[0];
                let name = parts[1];

                let source = crate::package_resolver::load_package(
                    namespace,
                    name,
                    &self.config.package_remote_url,
                    &self.config.package_remote_type,
                    self.config.package_access_token.as_deref(),
                )
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
                    println!(
                        "Package '{}' loaded into namespace '{}'",
                        package, namespace_key
                    );
                }

                Ok(ExecutionResult::Continue)
            }

            Statement::Group {
                name,
                severity,
                enabled,
                body,
                span: _,
            } => {
                let sev = severity.as_ref().unwrap_or(&Severity::Critical);

                if !enabled {
                    self.add_check(
                        name.clone(),
                        CheckStatus::Skipped,
                        Some("disabled".to_string()),
                        sev.clone(),
                    );
                    return Ok(ExecutionResult::Continue);
                }

                let result = self.execute_statements(body);

                match result {
                    Ok(exec_result) => {
                        self.add_check(name.clone(), CheckStatus::Passed, None, sev.clone());
                        Ok(exec_result)
                    }
                    Err(e) => {
                        self.add_check(
                            name.clone(),
                            CheckStatus::Failed,
                            Some(e.to_string()),
                            sev.clone(),
                        );
                        Err(e)
                    }
                }
            }

            Statement::Try {
                body,
                catch_var,
                catch_body,
                span: _,
            } => {
                let result = self.execute_statements(body);

                match result {
                    Ok(exec_result) => Ok(exec_result),
                    Err(e) => {
                        let var_name = catch_var.as_ref().map(|s| s.as_str()).unwrap_or("error");
                        self.variables
                            .insert(var_name.to_string(), Value::String(e.to_string()));

                        self.execute_statements(catch_body)
                    }
                }
            }
        }
    }

    fn execute_foreach(
        &mut self,
        collection: &Value,
        var_name: &str,
        pattern: Option<&str>,
        body: &[Statement],
        span: &Span,
    ) -> Result<ExecutionResult> {
        let items = match collection {
            Value::Array(arr) => arr,
            Value::Object(obj) if obj.type_name == "Git" => {
                if let Some(Value::Array(files)) = obj.get("files") {
                    files
                } else {
                    return Ok(ExecutionResult::Continue);
                }
            }
            _ => bail_span!(span, "Cannot iterate over {:?}", collection),
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

    fn execute_match(
        &mut self,
        subject: &Value,
        arms: &[(MatchPattern, Vec<Statement>)],
    ) -> Result<ExecutionResult> {
        for (pattern, body) in arms {
            if self.pattern_matches(pattern, subject)? {
                return self.execute_statements(body);
            }
        }

        Ok(ExecutionResult::Continue)
    }

    fn pattern_matches(&mut self, pattern: &MatchPattern, value: &Value) -> Result<bool> {
        match pattern {
            MatchPattern::Expression(expr, _) => {
                let pattern_val = self.eval_expression(expr)?;
                value.equals(&pattern_val)
            }

            MatchPattern::Wildcard(s, _) => {
                let value_str = value.as_string()?;
                let pattern_str = s.as_str();

                if pattern_str.contains('*') {
                    let regex_pattern = pattern_str.replace(".", "\\.").replace("*", ".*");
                    let regex = regex::Regex::new(&format!("^{}$", regex_pattern))?;
                    Ok(regex.is_match(&value_str))
                } else {
                    Ok(value_str == pattern_str)
                }
            }

            MatchPattern::Underscore(_) => Ok(true),
        }
    }

    fn run_command(&self, cmd: &str, span: &Span) -> Result<()> {
        if self.verbose {
            println!("> Running: {}", cmd);
        }

        let mut child = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context(format!("Failed to execute command: {}", cmd))?;

        let timeout = self.config.command_timeout;
        let start = std::time::Instant::now();

        loop {
            match child.try_wait() {
                Ok(Some(_status)) => {
                    let output = child.wait_with_output()?;
                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        bail_span!(span, "Command failed: {}\n{}", cmd, stderr);
                    }
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    if !stdout.is_empty() {
                        print!("{}", stdout);
                    }
                    return Ok(());
                }
                Ok(None) => {
                    if start.elapsed() > timeout {
                        let _ = child.kill();
                        bail_span!(
                            span,
                            "Command timed out after {} seconds: {}",
                            timeout.as_secs(),
                            cmd
                        );
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                Err(e) => bail_span!(span, "Error waiting for command: {}", e),
            }
        }
    }

    /// Run a single command inside a `parallel` block (called from rayon threads).
    fn run_parallel_command(
        cmd: &str,
        span: Span,
        verbose: bool,
        timeout: std::time::Duration,
    ) -> Result<std::process::Output> {
        if verbose {
            println!("> Running: {}", cmd);
        }

        let mut child = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to execute: {}", cmd))?;

        let start = std::time::Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(_)) => {
                    let output = child.wait_with_output()?;
                    if !output.status.success() {
                        bail_span!(
                            span,
                            "Command failed: {}\nstderr: {}",
                            cmd,
                            String::from_utf8_lossy(&output.stderr)
                        );
                    }
                    return Ok(output);
                }
                Ok(None) => {
                    if start.elapsed() > timeout {
                        let _ = child.kill();
                        bail_span!(
                            span,
                            "Command timed out after {} seconds: {}",
                            timeout.as_secs(),
                            cmd
                        );
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                Err(e) => bail_span!(span, "Error waiting for command: {}", e),
            }
        }
    }

    fn matches_pattern(text: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        if pattern.starts_with('*') && pattern.ends_with('*') {
            let middle = &pattern[1..pattern.len() - 1];
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
    use githook_syntax::ast::{BinaryOp, Expression};
    use githook_syntax::error::Span;
    use githook_syntax::{lexer, parser};

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
        let mut executor = Executor::new();

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
        let mut executor = Executor::new();

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
        let mut executor = Executor::new();

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
        let mut executor = Executor::new();
        let expr = Expression::String("hello".to_string(), dummy_span());
        let result = executor.eval_expression(&expr).unwrap();

        assert!(matches!(result, Value::String(s) if s == "hello"));
    }

    #[test]
    fn test_eval_number_literal() {
        let mut executor = Executor::new();
        let expr = Expression::Number(3.15, dummy_span());
        let result = executor.eval_expression(&expr).unwrap();

        assert!(matches!(result, Value::Number(n) if (n - 3.15).abs() < 0.001));
    }

    #[test]
    fn test_eval_bool_literal() {
        let mut executor = Executor::new();
        let expr = Expression::Bool(true, dummy_span());
        let result = executor.eval_expression(&expr).unwrap();

        assert!(matches!(result, Value::Bool(true)));
    }

    #[test]
    fn test_eval_null_literal() {
        let mut executor = Executor::new();
        let expr = Expression::Null(dummy_span());
        let result = executor.eval_expression(&expr).unwrap();

        assert!(matches!(result, Value::Null));
    }

    #[test]
    fn test_variable_not_found() {
        let mut executor = Executor::new();
        let expr = Expression::Identifier("unknown".to_string(), dummy_span());
        let result = executor.eval_expression(&expr);

        assert!(result.is_err());
    }

    #[test]
    fn test_set_and_get_variable() {
        let mut executor = Executor::new();
        executor.set_variable("test".to_string(), Value::Number(100.0));

        assert!(executor.variables.contains_key("test"));
        assert!(matches!(
            executor.variables.get("test"),
            Some(Value::Number(100.0))
        ));
    }

    #[test]
    fn test_matches_pattern() {
        assert!(Executor::matches_pattern("test.rs", "*.rs"));
        assert!(Executor::matches_pattern("test.rs", "test.*"));
        assert!(Executor::matches_pattern("test.rs", "*est*"));
        assert!(Executor::matches_pattern("anything", "*"));
        assert!(!Executor::matches_pattern("test.py", "*.rs"));
    }

    // ── If / Else ─────────────────────────────────────────────

    #[test]
    fn test_if_true_branch() {
        let input = r#"
            let x = 0
            if true {
                let x = 42
            }
        "#;
        let executor = parse_and_execute(input).unwrap();
        assert!(matches!(executor.variables.get("x"), Some(Value::Number(n)) if *n == 42.0));
    }

    #[test]
    fn test_if_false_branch() {
        let input = r#"
            let x = 1
            if false {
                let x = 99
            }
        "#;
        let executor = parse_and_execute(input).unwrap();
        assert!(matches!(executor.variables.get("x"), Some(Value::Number(n)) if *n == 1.0));
    }

    #[test]
    fn test_if_else_true() {
        let input = r#"
            let x = 0
            if true {
                let x = 1
            } else {
                let x = 2
            }
        "#;
        let executor = parse_and_execute(input).unwrap();
        assert!(matches!(executor.variables.get("x"), Some(Value::Number(n)) if *n == 1.0));
    }

    #[test]
    fn test_if_else_false() {
        let input = r#"
            let x = 0
            if false {
                let x = 1
            } else {
                let x = 2
            }
        "#;
        let executor = parse_and_execute(input).unwrap();
        assert!(matches!(executor.variables.get("x"), Some(Value::Number(n)) if *n == 2.0));
    }

    // ── Try / Catch ───────────────────────────────────────────

    #[test]
    fn test_try_no_error() {
        let input = r#"
            let x = 0
            try {
                let x = 42
            } catch {
                let x = -1
            }
        "#;
        let executor = parse_and_execute(input).unwrap();
        assert!(matches!(executor.variables.get("x"), Some(Value::Number(n)) if *n == 42.0));
    }

    #[test]
    fn test_try_catches_error() {
        // Referencing an undefined variable inside try should trigger catch
        let input = r#"
            let caught = false
            try {
                let x = undefined_var
            } catch {
                let caught = true
            }
        "#;
        let executor = parse_and_execute(input).unwrap();
        assert!(matches!(
            executor.variables.get("caught"),
            Some(Value::Bool(true))
        ));
    }

    #[test]
    fn test_try_catch_error_variable() {
        let input = r#"
            let msg = ""
            try {
                let x = undefined_var
            } catch { err in
                let msg = err
            }
        "#;
        let executor = parse_and_execute(input).unwrap();
        if let Some(Value::String(s)) = executor.variables.get("msg") {
            assert!(!s.is_empty(), "Error message should not be empty");
        } else {
            panic!("Expected msg to be a non-empty string");
        }
    }

    // ── Match ─────────────────────────────────────────────────

    #[test]
    fn test_match_exact() {
        let input = r#"
            let result = 0
            let x = "hello"
            match x {
                "hello" -> { let result = 1 }
                "world" -> { let result = 2 }
                _ -> { let result = 3 }
            }
        "#;
        let executor = parse_and_execute(input).unwrap();
        assert!(matches!(executor.variables.get("result"), Some(Value::Number(n)) if *n == 1.0));
    }

    #[test]
    fn test_match_wildcard_pattern() {
        let input = r#"
            let result = 0
            let x = "feature/xyz"
            match x {
                "main" -> { let result = 1 }
                "feature/*" -> { let result = 2 }
                _ -> { let result = 3 }
            }
        "#;
        let executor = parse_and_execute(input).unwrap();
        assert!(matches!(executor.variables.get("result"), Some(Value::Number(n)) if *n == 2.0));
    }

    #[test]
    fn test_match_underscore_fallback() {
        let input = r#"
            let result = 0
            let x = "other"
            match x {
                "main" -> { let result = 1 }
                "develop" -> { let result = 2 }
                _ -> { let result = 99 }
            }
        "#;
        let executor = parse_and_execute(input).unwrap();
        assert!(matches!(executor.variables.get("result"), Some(Value::Number(n)) if *n == 99.0));
    }

    // ── Macros ────────────────────────────────────────────────

    #[test]
    fn test_macro_def_and_call() {
        let input = r#"
            macro set_x() {
                let x = 42
            }
            @set_x()
        "#;
        let executor = parse_and_execute(input).unwrap();
        assert!(matches!(executor.variables.get("x"), Some(Value::Number(n)) if *n == 42.0));
    }

    #[test]
    fn test_macro_with_params() {
        let input = r#"
            macro set_val(name, val) {
                let result = val
            }
            @set_val("ignored", 99)
        "#;
        let executor = parse_and_execute(input).unwrap();
        assert!(matches!(executor.variables.get("result"), Some(Value::Number(n)) if *n == 99.0));
    }

    #[test]
    fn test_macro_undefined_error() {
        let input = r#"@nonexistent()"#;
        let result = parse_and_execute(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_macro_wrong_arg_count() {
        let input = r#"
            macro greet(name) {
                print name
            }
            @greet("a", "b")
        "#;
        let result = parse_and_execute(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_macro_params_dont_leak() {
        // Macro parameters should not be visible after the call
        let input = r#"
            macro set(val) {
                let result = val
            }
            @set(42)
        "#;
        let executor = parse_and_execute(input).unwrap();
        // "result" set by macro body should be visible
        assert!(matches!(executor.variables.get("result"), Some(Value::Number(n)) if *n == 42.0));
        // "val" parameter should NOT leak into caller scope
        assert!(!executor.variables.contains_key("val"));
    }

    #[test]
    fn test_macro_preserves_existing_var_with_same_name_as_param() {
        // If caller has a variable with the same name as a macro param,
        // it should be restored after the macro call
        let input = r#"
            let val = 100
            macro set(val) {
                let result = val
            }
            @set(42)
        "#;
        let executor = parse_and_execute(input).unwrap();
        assert!(matches!(executor.variables.get("result"), Some(Value::Number(n)) if *n == 42.0));
        // "val" should be restored to 100, not 42
        assert!(matches!(executor.variables.get("val"), Some(Value::Number(n)) if *n == 100.0));
    }

    // ── ForEach ───────────────────────────────────────────────

    #[test]
    fn test_foreach_array() {
        let input = r#"
            let sum = 0
            let items = [1, 2, 3]
            foreach items {
                item in
                let sum = sum + item
            }
        "#;
        let executor = parse_and_execute(input).unwrap();
        assert!(matches!(executor.variables.get("sum"), Some(Value::Number(n)) if *n == 6.0));
    }

    #[test]
    fn test_foreach_empty_array() {
        let input = r#"
            let sum = 0
            let items = []
            foreach items {
                item in
                let sum = sum + 1
            }
        "#;
        let executor = parse_and_execute(input).unwrap();
        assert!(matches!(executor.variables.get("sum"), Some(Value::Number(n)) if *n == 0.0));
    }

    #[test]
    fn test_foreach_non_iterable_error() {
        let input = r#"
            let x = 42
            foreach x {
                item in
                print item
            }
        "#;
        let result = parse_and_execute(input);
        assert!(result.is_err());
    }

    // ── Block If / Warn If ────────────────────────────────────

    #[test]
    fn test_block_if_true() {
        let input = r#"block if true message "blocked!""#;
        let executor = parse_and_execute(input).unwrap();
        assert!(!executor.blocks.is_empty());
        assert_eq!(executor.blocks[0], "blocked!");
    }

    #[test]
    fn test_block_if_false() {
        let input = r#"block if false message "blocked!""#;
        let executor = parse_and_execute(input).unwrap();
        assert!(executor.blocks.is_empty());
    }

    #[test]
    fn test_warn_if_true() {
        let input = r#"warn if true message "warning!""#;
        let executor = parse_and_execute(input).unwrap();
        assert!(!executor.warnings.is_empty());
        assert_eq!(executor.warnings[0], "warning!");
    }

    #[test]
    fn test_warn_if_false() {
        let input = r#"warn if false message "warning!""#;
        let executor = parse_and_execute(input).unwrap();
        assert!(executor.warnings.is_empty());
    }

    // ── Group ─────────────────────────────────────────────────

    #[test]
    fn test_group_passing() {
        let input = r#"
            group formatting critical {
                let x = 1
            }
        "#;
        let executor = parse_and_execute(input).unwrap();
        assert!(
            executor
                .check_results
                .iter()
                .any(|c| c.name == "formatting" && c.status == CheckStatus::Passed)
        );
    }

    #[test]
    fn test_group_disabled() {
        let input = r#"
            group slow_tests critical disabled {
                let x = undefined_var
            }
        "#;
        // Disabled groups don't execute their body — no error even if body would fail
        let executor = parse_and_execute(input).unwrap();
        assert!(
            executor
                .check_results
                .iter()
                .any(|c| c.name == "slow_tests" && c.status == CheckStatus::Skipped)
        );
    }

    // ── Expressions ───────────────────────────────────────────

    #[test]
    fn test_unary_not() {
        let mut executor = Executor::new();
        let expr = Expression::Unary {
            op: githook_syntax::ast::UnaryOp::Not,
            expr: Box::new(Expression::Bool(true, dummy_span())),
            span: dummy_span(),
        };
        let result = executor.eval_expression(&expr).unwrap();
        assert!(matches!(result, Value::Bool(false)));
    }

    #[test]
    fn test_unary_minus() {
        let mut executor = Executor::new();
        let expr = Expression::Unary {
            op: githook_syntax::ast::UnaryOp::Minus,
            expr: Box::new(Expression::Number(5.0, dummy_span())),
            span: dummy_span(),
        };
        let result = executor.eval_expression(&expr).unwrap();
        assert!(matches!(result, Value::Number(n) if (n - (-5.0)).abs() < f64::EPSILON));
    }

    #[test]
    fn test_array_literal() {
        let mut executor = Executor::new();
        let expr = Expression::Array(
            vec![
                Expression::Number(1.0, dummy_span()),
                Expression::Number(2.0, dummy_span()),
            ],
            dummy_span(),
        );
        let result = executor.eval_expression(&expr).unwrap();
        assert!(matches!(result, Value::Array(ref arr) if arr.len() == 2));
    }

    #[test]
    fn test_string_equality() {
        let input = r#"
            let a = "hello"
            let b = "hello"
            let eq = a == b
        "#;
        let executor = parse_and_execute(input).unwrap();
        assert!(matches!(
            executor.variables.get("eq"),
            Some(Value::Bool(true))
        ));
    }

    #[test]
    fn test_string_inequality() {
        let input = r#"
            let a = "hello"
            let b = "world"
            let neq = a != b
        "#;
        let executor = parse_and_execute(input).unwrap();
        assert!(matches!(
            executor.variables.get("neq"),
            Some(Value::Bool(true))
        ));
    }

    #[test]
    fn test_arithmetic_operations() {
        let input = r#"
            let add = 10 + 5
            let sub = 10 - 5
            let mul = 10 * 5
            let div = 10 / 5
        "#;
        let executor = parse_and_execute(input).unwrap();
        assert!(matches!(executor.variables.get("add"), Some(Value::Number(n)) if *n == 15.0));
        assert!(matches!(executor.variables.get("sub"), Some(Value::Number(n)) if *n == 5.0));
        assert!(matches!(executor.variables.get("mul"), Some(Value::Number(n)) if *n == 50.0));
        assert!(matches!(executor.variables.get("div"), Some(Value::Number(n)) if *n == 2.0));
    }

    // ── Ternary if-then-else expressions ──────────────────────

    #[test]
    fn test_ternary_true_branch() {
        let input = r#"
            let result = if true then "yes" else "no"
        "#;
        let executor = parse_and_execute(input).unwrap();
        assert!(matches!(
            executor.variables.get("result"),
            Some(Value::String(s)) if s == "yes"
        ));
    }

    #[test]
    fn test_ternary_false_branch() {
        let input = r#"
            let result = if false then "yes" else "no"
        "#;
        let executor = parse_and_execute(input).unwrap();
        assert!(matches!(
            executor.variables.get("result"),
            Some(Value::String(s)) if s == "no"
        ));
    }

    #[test]
    fn test_ternary_with_comparison() {
        let input = r#"
            let x = 10
            let label = if x > 5 then "big" else "small"
        "#;
        let executor = parse_and_execute(input).unwrap();
        assert!(matches!(
            executor.variables.get("label"),
            Some(Value::String(s)) if s == "big"
        ));
    }

    #[test]
    fn test_ternary_with_number_result() {
        let input = r#"
            let x = 3
            let result = if x > 0 then x * 2 else 0
        "#;
        let executor = parse_and_execute(input).unwrap();
        assert!(matches!(
            executor.variables.get("result"),
            Some(Value::Number(n)) if *n == 6.0
        ));
    }

    #[test]
    fn test_ternary_nested() {
        let input = r#"
            let x = 2
            let label = if x > 5 then "big" else if x > 0 then "small" else "zero"
        "#;
        let executor = parse_and_execute(input).unwrap();
        assert!(matches!(
            executor.variables.get("label"),
            Some(Value::String(s)) if s == "small"
        ));
    }

    #[test]
    fn test_ternary_in_print() {
        let input = r#"
            let active = true
            print if active then "on" else "off"
        "#;
        // Should not error — print accepts an expression
        assert!(parse_and_execute(input).is_ok());
    }

    #[test]
    fn test_ternary_in_condition() {
        let input = r#"
            let x = 1
            let flag = if x == 1 then true else false
            if flag {
                let result = "ok"
            }
        "#;
        let executor = parse_and_execute(input).unwrap();
        assert!(matches!(
            executor.variables.get("result"),
            Some(Value::String(s)) if s == "ok"
        ));
    }

    #[test]
    fn test_ternary_with_string_concat() {
        let input = r#"
            let prefix = if true then "hello" else "bye"
            let msg = prefix + " world"
        "#;
        let executor = parse_and_execute(input).unwrap();
        assert!(matches!(
            executor.variables.get("msg"),
            Some(Value::String(s)) if s == "hello world"
        ));
    }

    // ── Index / bracket access tests ─────────────────────────────────

    #[test]
    fn test_index_access_on_array() {
        let input = r#"
            let arr = [10, 20, 30]
            let first = arr[0]
            let second = arr[1]
            let third = arr[2]
        "#;
        let executor = parse_and_execute(input).unwrap();
        assert!(matches!(executor.variables.get("first"), Some(Value::Number(n)) if *n == 10.0));
        assert!(matches!(executor.variables.get("second"), Some(Value::Number(n)) if *n == 20.0));
        assert!(matches!(executor.variables.get("third"), Some(Value::Number(n)) if *n == 30.0));
    }

    #[test]
    fn test_index_access_on_array_out_of_bounds() {
        let input = r#"
            let arr = [1, 2]
            let x = arr[5]
        "#;
        let result = parse_and_execute(input);
        assert!(result.is_err());
        let err = format!("{}", result.err().unwrap());
        assert!(err.contains("out of bounds"), "Error was: {err}");
    }

    #[test]
    fn test_index_access_on_string() {
        let input = r#"
            let s = "hello"
            let ch = s[1]
        "#;
        let executor = parse_and_execute(input).unwrap();
        assert!(matches!(
            executor.variables.get("ch"),
            Some(Value::String(s)) if s == "e"
        ));
    }

    #[test]
    fn test_index_access_with_expression_index() {
        let input = r#"
            let arr = [10, 20, 30]
            let i = 1
            let val = arr[i]
        "#;
        let executor = parse_and_execute(input).unwrap();
        assert!(matches!(executor.variables.get("val"), Some(Value::Number(n)) if *n == 20.0));
    }

    #[test]
    fn test_index_access_with_computed_index() {
        let input = r#"
            let arr = [10, 20, 30]
            let val = arr[1 + 1]
        "#;
        let executor = parse_and_execute(input).unwrap();
        assert!(matches!(executor.variables.get("val"), Some(Value::Number(n)) if *n == 30.0));
    }

    #[test]
    fn test_index_access_on_dict() {
        let mut executor = Executor::new();
        let mut dict = crate::value::Object::new("Dict");
        dict.set("name", Value::String("Alice".to_string()));
        dict.set("age", Value::Number(30.0));
        executor
            .variables
            .insert("person".to_string(), Value::Object(dict));

        let tokens = lexer::tokenize(r#"let n = person["name"]"#).unwrap();
        let ast = parser::parse(tokens).unwrap();
        for stmt in ast {
            executor.execute_statement(&stmt).unwrap();
        }
        assert!(matches!(
            executor.variables.get("n"),
            Some(Value::String(s)) if s == "Alice"
        ));
    }

    #[test]
    fn test_index_access_on_dict_number_value() {
        let mut executor = Executor::new();
        let mut dict = crate::value::Object::new("Dict");
        dict.set("count", Value::Number(42.0));
        executor
            .variables
            .insert("data".to_string(), Value::Object(dict));

        let tokens = lexer::tokenize(r#"let c = data["count"]"#).unwrap();
        let ast = parser::parse(tokens).unwrap();
        for stmt in ast {
            executor.execute_statement(&stmt).unwrap();
        }
        assert!(matches!(
            executor.variables.get("c"),
            Some(Value::Number(n)) if *n == 42.0
        ));
    }

    #[test]
    fn test_index_access_on_dict_missing_key() {
        let mut executor = Executor::new();
        let dict = crate::value::Object::new("Dict");
        executor
            .variables
            .insert("data".to_string(), Value::Object(dict));

        let tokens = lexer::tokenize(r#"let x = data["missing"]"#).unwrap();
        let ast = parser::parse(tokens).unwrap();
        let result = executor.execute_statement(&ast[0]);
        assert!(result.is_err());
        let err = format!("{}", result.err().unwrap());
        assert!(err.contains("not found"), "Error was: {err}");
    }

    #[test]
    fn test_index_access_chained_with_dot() {
        let mut executor = Executor::new();
        let mut inner = crate::value::Object::new("Dict");
        inner.set("x", Value::Number(99.0));
        let mut outer = crate::value::Object::new("Dict");
        outer.set("inner", Value::Object(inner));
        executor
            .variables
            .insert("obj".to_string(), Value::Object(outer));

        let tokens = lexer::tokenize(r#"let val = obj["inner"]["x"]"#).unwrap();
        let ast = parser::parse(tokens).unwrap();
        for stmt in ast {
            executor.execute_statement(&stmt).unwrap();
        }
        assert!(matches!(
            executor.variables.get("val"),
            Some(Value::Number(n)) if *n == 99.0
        ));
    }

    #[test]
    fn test_index_access_array_of_dicts() {
        let mut executor = Executor::new();
        let mut d1 = crate::value::Object::new("Dict");
        d1.set("id", Value::Number(1.0));
        let mut d2 = crate::value::Object::new("Dict");
        d2.set("id", Value::Number(2.0));
        executor.variables.insert(
            "items".to_string(),
            Value::Array(vec![Value::Object(d1), Value::Object(d2)]),
        );

        let tokens = lexer::tokenize(r#"let first_id = items[0]["id"]"#).unwrap();
        let ast = parser::parse(tokens).unwrap();
        for stmt in ast {
            executor.execute_statement(&stmt).unwrap();
        }
        assert!(matches!(
            executor.variables.get("first_id"),
            Some(Value::Number(n)) if *n == 1.0
        ));
    }

    #[test]
    fn test_index_access_nested_json_structure() {
        // Simulate a JSON-parsed structure: {"users": [{"name": "Bob"}]}
        let mut executor = Executor::new();
        let mut user = crate::value::Object::new("Dict");
        user.set("name", Value::String("Bob".to_string()));
        let mut root = crate::value::Object::new("Dict");
        root.set("users", Value::Array(vec![Value::Object(user)]));
        executor
            .variables
            .insert("data".to_string(), Value::Object(root));

        let tokens = lexer::tokenize(r#"let name = data["users"][0]["name"]"#).unwrap();
        let ast = parser::parse(tokens).unwrap();
        for stmt in ast {
            executor.execute_statement(&stmt).unwrap();
        }
        assert!(matches!(
            executor.variables.get("name"),
            Some(Value::String(s)) if s == "Bob"
        ));
    }

    #[test]
    fn test_index_access_with_variable_key() {
        let mut executor = Executor::new();
        let mut dict = crate::value::Object::new("Dict");
        dict.set("hello", Value::String("world".to_string()));
        executor
            .variables
            .insert("data".to_string(), Value::Object(dict));
        executor
            .variables
            .insert("key".to_string(), Value::String("hello".to_string()));

        let tokens = lexer::tokenize(r#"let val = data[key]"#).unwrap();
        let ast = parser::parse(tokens).unwrap();
        for stmt in ast {
            executor.execute_statement(&stmt).unwrap();
        }
        assert!(matches!(
            executor.variables.get("val"),
            Some(Value::String(s)) if s == "world"
        ));
    }
}
