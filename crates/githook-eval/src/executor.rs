use crate::context::ExecutionContext;
use crate::conditions::evaluate_block_condition;
use githook_syntax::{
    Argument, ContentCheck, ContentScope, MessageCheck, RuleSeverity, Statement,
    MatchSubject, MatchArm, MatchPattern
};
use anyhow::{Result, bail, Context as AnyhowContext};
use colored::*;
use regex::Regex;
use std::process::Command;
use std::borrow::Cow;
use std::sync::OnceLock;

#[derive(Debug, PartialEq)]
pub enum ExecutionStatus {
    Ok,
    Warn,
    Block,
}

static GLOB_CACHE: OnceLock<std::sync::Mutex<lru::LruCache<String, glob::Pattern>>> = OnceLock::new();

fn get_cached_glob(pattern: &str) -> Result<glob::Pattern> {
    let cache = GLOB_CACHE.get_or_init(|| {
        std::sync::Mutex::new(
            lru::LruCache::new(
                std::num::NonZeroUsize::new(128)
                    .expect("128 is a valid non-zero cache size")
            )
        )
    });

    let mut cache = cache.lock()
        .expect("Glob cache mutex should not be poisoned");
    
    if let Some(glob_pattern) = cache.get(pattern) {
        return Ok(glob_pattern.clone());
    }

    let glob_pattern = glob::Pattern::new(pattern)?;
    cache.put(pattern.to_string(), glob_pattern.clone());
    Ok(glob_pattern)
}
fn substitute_placeholders<'a>(input: &'a str, context: &ExecutionContext) -> Cow<'a, str> {
    if !input.contains('{') {
        return Cow::Borrowed(input);
    }

    let mut out = input.to_string();
    let mut changed = false;

    let re = regex::Regex::new(r"\{([a-z]+):([a-zA-Z0-9_]+)(\|[a-z0-9_:]+)*\}")
        .expect("Valid regex pattern for placeholder matching");
    
    for cap in re.captures_iter(input) {
        let full_match = &cap[0];
        let namespace = &cap[1];
        let key = &cap[2];
        let filters_str = cap.get(3).map(|m| m.as_str()).unwrap_or("");
        
        if let Some(mut value) = context.placeholder_registry().resolve(namespace, key, context) {
            if !filters_str.is_empty() {
                let filters: Vec<&str> = filters_str.split('|').filter(|s| !s.is_empty()).collect();
                for filter in filters {
                    value = apply_filter(&value, filter);
                }
            }
            
            out = out.replace(full_match, &value);
            changed = true;
        }
    }

    for (k, v) in context.vars() {
        let placeholder = format!("{{{}}}", k);
        if out.contains(&placeholder) {
            out = out.replace(&placeholder, v);
            changed = true;
        }
    }

    if changed {
        Cow::Owned(out)
    } else {
        Cow::Borrowed(input)
    }
}

fn apply_filter(value: &str, filter: &str) -> String {
    if filter == "upper" {
        value.to_uppercase()
    } else if filter == "lower" {
        value.to_lowercase()
    } else if filter == "trim" {
        value.trim().to_string()
    } else if filter == "len" {
        value.len().to_string()
    } else if filter.starts_with("truncate:") {
        let n: usize = filter.strip_prefix("truncate:").and_then(|s| s.parse().ok()).unwrap_or(10);
        value.chars().take(n).collect()
    } else if filter.starts_with("replace:") {
        let parts: Vec<&str> = filter.strip_prefix("replace:").unwrap_or("").split(':').collect();
        if parts.len() >= 2 {
            value.replace(parts[0], parts[1])
        } else {
            value.to_string()
        }
    } else {
        value.to_string()
    }
}

fn parse_command(cmd: &str) -> Result<(String, Vec<String>)> {
    let args = shell_words::split(cmd)
        .map_err(|e| anyhow::anyhow!("Failed to parse command '{}': {}", cmd, e))?;
    
    if args.is_empty() {
        return Ok((String::new(), vec![]));
    }
    
    Ok((args[0].clone(), args[1..].to_vec()))
}

pub fn execute(statements: Vec<Statement>, hook_args: &[String]) -> Result<ExecutionStatus> {
    execute_with_filters(statements, hook_args, None, None)
}

pub fn execute_with_filters(
    statements: Vec<Statement>,
    hook_args: &[String],
    allowed_groups: Option<Vec<String>>,
    skipped_groups: Option<Vec<String>>,
) -> Result<ExecutionStatus> {
    let mut context = ExecutionContext::new_with_filters(allowed_groups, skipped_groups);

    for statement in &statements {
        if !execute_statement(statement, &mut context, hook_args)? {
            print_summary(&context);
            return Ok(ExecutionStatus::Block);
        }
    }

    print_summary(&context);

    if context.has_warnings() {
        Ok(ExecutionStatus::Warn)
    } else {
        Ok(ExecutionStatus::Ok)
    }
}

fn print_summary(ctx: &ExecutionContext) {
    println!("\n{}", "═".repeat(50));

    let checks = ctx.checks_run();

    if checks == 0 {
        println!("o {} checks completed", checks);
    } else {
        println!("o {} check{} completed", checks, if checks == 1 { "" } else { "s" });
    }

    if !ctx.checks_passed().is_empty() {
        println!("\no Passed checks:");
        for check in ctx.checks_passed() {
            println!("  - {}", check);
        }
    }

    if ctx.has_warnings() {
        println!("\n{} Warnings:", "!".yellow());
        for (warning, locations) in ctx.warnings() {
            println!("  - {}", warning.yellow());
            for loc in locations {
                println!("    in {}", loc.dimmed());
            }
        }
    }

    println!("{}", "═".repeat(50));
}

fn execute_run(cmd: &str, context: &mut ExecutionContext) -> Result<bool> {
    let rendered_cmd = substitute_placeholders(cmd, context);
    
    if !context.is_command_allowed(&rendered_cmd) {
        println!("  {} Command '{}' is not in allow list", "x".red(), rendered_cmd.red());
        return Ok(false);
    }
    context.check_run();

    let (program, args) = match parse_command(&rendered_cmd) {
        Ok(parsed) => parsed,
        Err(e) => {
            println!("  {} Failed to parse command: {}", "x".red(), e);
            return Ok(false);
        }
    };
    
    if program.is_empty() {
        println!("  {} Empty command", "x".red());
        return Ok(false);
    }

    let output = Command::new(&program)
        .args(&args)
        .output()?;

    if !output.status.success() {
        println!("  {} Command failed: {}", "x".red(), rendered_cmd.red());
        if !output.stderr.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            for line in stderr.lines() {
                println!("    {}", line.dimmed());
            }
        }
        return Ok(false);
    }
    context.check_passed(format!("Command: {}", rendered_cmd));
    Ok(true)
}

fn execute_bool_literal(value: bool, context: &mut ExecutionContext) -> Result<bool> {
    context.check_run();
    if value {
        context.check_passed("literal true".to_string());
        Ok(true)
    } else {
        Ok(false)
    }
}

fn execute_group(definition: &githook_syntax::GroupDefinition, context: &mut ExecutionContext, hook_args: &[String]) -> Result<bool> {
    if let Some(allowed) = context.allowed_groups()
        && !allowed.contains(&definition.name)
    {
        return Ok(true);
    }
    
    if let Some(skipped) = context.skipped_groups()
        && skipped.contains(&definition.name)
    {
        return Ok(true);
    }
    
    let is_enabled = definition.enabled.unwrap_or(true);
    
    if !is_enabled {
        return Ok(true);
    }
    
    let severity_str = match &definition.severity {
        Some(githook_syntax::GroupSeverity::Critical(_)) => "CRITICAL",
        Some(githook_syntax::GroupSeverity::Warning(_)) => "WARNING",
        Some(githook_syntax::GroupSeverity::Info(_)) => "INFO",
        None => "GROUP",
    };
    
    println!("\n{} [{}]", format!("- {}", definition.name).cyan().bold(), severity_str.yellow());
    
    let mut all_passed = true;
    for stmt in &definition.body {
        if !execute_statement(stmt, context, hook_args)? {
            all_passed = false;
        }
    }
    
    if all_passed {
        println!("{} Group '{}' passed", "o".green().bold(), definition.name.green());
    } else {
        println!("{} Group '{}' failed", "x".red().bold(), definition.name.red());
    }
    
    Ok(all_passed)
}

fn execute_let_string_list(name: String, items: Vec<String>, context: &mut ExecutionContext) -> Result<bool> {
    context.set_string_list(name, items);
    Ok(true)
}

fn execute_block(msg: &str) -> Result<bool> {
    println!("  {} {}", "x".red().bold(), msg.red());
    Ok(false)
}

fn execute_foreach_string_list(
    var: &str,
    list: &str,
    body: &[Statement],
    context: &mut ExecutionContext,
    hook_args: &[String]
) -> Result<bool> {
    let items = match context.get_string_list(list) {
        Some(xs) => xs.to_vec(),
        None => {
            println!("  {} unknown string list '{}'", "x".red(), list.red());
            return Ok(false);
        }
    };
    
    for item in items { 
        context.set_var(var.to_string(), item); 
        for stmt in body {
            if !execute_statement(stmt, context, hook_args)? {
                context.unset_var(var);
                return Ok(false);
            }
        }
    }
    context.unset_var(var);
    context.check_passed(format!("foreach {} in {}", var, list));
    Ok(true)
}

fn execute_foreach_array(
    var: &str,
    items: &[githook_syntax::Argument],
    body: &[Statement],
    context: &mut ExecutionContext,
    hook_args: &[String]
) -> Result<bool> {
    for item in items {
        let value = match item {
            githook_syntax::Argument::String(s, _) => s.clone(),
            githook_syntax::Argument::Number(n, _) => n.to_string(),
            githook_syntax::Argument::Identifier(id, _) => {
                context.get_var(id).map(|s| s.to_string()).unwrap_or_else(|| id.clone())
            },
            githook_syntax::Argument::Array(_, _) => {
                println!("  {} nested arrays not supported in foreach", "x".red());
                return Ok(false);
            }
        };
        
        context.set_var(var.to_string(), value);
        
        for stmt in body {
            if !execute_statement(stmt, context, hook_args)? {
                context.unset_var(var);
                return Ok(false);
            }
        }
    }
    
    context.unset_var(var);
    context.check_passed(format!("foreach {} in [...]", var));
    Ok(true)
}

fn execute_foreach_staged_files(
    var: &str,
    pattern: &str,
    where_cond: &Option<githook_syntax::BlockCondition>,
    body: &[Statement],
    context: &mut ExecutionContext,
    hook_args: &[String]
) -> Result<bool> {
    let files = context.staged_files(pattern)?;

    if files.is_empty() {
        return Ok(true);
    }

    for file in files {
        context.enter_file(file.clone());
        context.set_var(var.to_string(), file);

        if let Some(cond) = where_cond
            && !evaluate_block_condition(cond, context, hook_args)?
        {
            context.unset_var(var);
            context.leave_file();
            continue;
        }

        for statement in body {
            if !execute_statement(statement, context, hook_args)? {
                context.unset_var(var);
                context.leave_file();
                return Ok(false);
            }
        }

        context.leave_file();
    }

    context.unset_var(var);
    context.check_passed(format!("foreach {} in staged_files matching '{}'", var, pattern));
    Ok(true)
}

fn execute_parallel(commands: &[String], context: &mut ExecutionContext) -> Result<bool> {
    use std::sync::{Arc, Mutex};
    use std::thread;

    context.check_run();

    let results = Arc::new(Mutex::new(Vec::new()));
    let mut handles = vec![];

    for cmd in commands {
        let cmd_str = substitute_placeholders(cmd, context).into_owned();
        
        let (program, args) = match parse_command(&cmd_str) {
            Ok(parsed) => parsed,
            Err(e) => {
                let mut res = results.lock()
                    .expect("Parallel results mutex should not be poisoned");
                res.push((cmd_str, false, Err(std::io::Error::other(format!("parse error: {}", e)))));
                continue;
            }
        };
        
        if program.is_empty() {
            let mut res = results.lock()
                .expect("Parallel results mutex should not be poisoned");
            res.push((cmd_str, false, Err(std::io::Error::other("empty command"))));
            continue;
        }

        let results = Arc::clone(&results);

        let handle = thread::spawn(move || {
            let output = Command::new(&program).args(&args).output();

            let success = match output {
                Ok(ref out) => out.status.success(),
                Err(_) => false,
            };

            let mut res = results.lock()
                .expect("Parallel results mutex should not be poisoned");
            res.push((cmd_str, success, output));
        });

        handles.push(handle);
    }

    let mut joins_ok = true;
    for handle in handles {
        if handle.join().is_err() {
            joins_ok = false;
        }
    }

    let results = results.lock()
        .expect("Parallel results mutex should not be poisoned");
    let mut all_passed = true;

    if !joins_ok || results.len() != commands.len() {
        all_passed = false;
        println!("  {} Parallel execution failed", "x".red());
    }

    for (cmd, success, output) in results.iter() {
        if !*success {
            println!("  {} {}", "x".red(), cmd.red());
            all_passed = false;

            if let Ok(output) = output
                && !output.stderr.is_empty()
            {
                let stderr = String::from_utf8_lossy(&output.stderr);
                for line in stderr.lines() {
                    println!("    {}", line.dimmed());
                }
            }
        }
    }

    if !all_passed {
        return Ok(false);
    }

    context.check_passed(format!("Parallel execution of {} commands", commands.len()));
    Ok(true)
}

fn execute_staged_content_validation(
    must: bool,
    check: &ContentCheck,
    pattern: &Option<String>,
    context: &mut ExecutionContext
) -> Result<bool> {
    context.check_run();

    let files = if let Some(pattern) = pattern {
        context.staged_files(pattern)?
    } else if let Some(file) = context.current_file() {
        vec![file.to_string()]
    } else {
        context.staged_files("*")?
    };

    let mut content = String::new();
    for file in &files {
        let file_content = githook_git::get_staged_file_content_from_index(file)?;
        content.push_str(&file_content);
        content.push('\n');
    }

    let result = match check {
        ContentCheck::Match(pattern, _) => {
            let regex = Regex::new(pattern)?;
            regex.is_match(&content)
        }
        ContentCheck::Contain(text, _) => content.contains(text),
    };

    let passed = if must { result } else { !result };

    if !passed {
        let check_desc = match check {
            ContentCheck::Match(p, _) => format!("match pattern '{}'", p),
            ContentCheck::Contain(t, _) => format!("contain '{}'", t),
        };
        let must_str = if must { "must" } else { "must not" };
        println!("  {} staged_content {} {}", "x".red(), must_str.red(), check_desc.red());
        return Ok(false);
    }

    let check_desc = match check {
        ContentCheck::Match(p, _) => format!(
            "staged_content must{} match '{}'",
            if must { "" } else { " not" },
            p
        ),
        ContentCheck::Contain(t, _) => format!(
            "staged_content must{} contain '{}'",
            if must { "" } else { " not" },
            t
        ),
    };
    context.check_passed(check_desc);
    Ok(true)
}

fn execute_staged_content_foreach(pattern: &str, body: &[Statement], context: &mut ExecutionContext, hook_args: &[String]) -> Result<bool> {
    let files = context.staged_files(pattern)?;

    if files.is_empty() {
        return Ok(true);
    }

    for file in files {
        context.set_current_file(Some(file));
        for stmt in body {
            if !execute_statement(stmt, context, hook_args)? {
                context.set_current_file(None);
                return Ok(false);
            }
        }
    }

    context.set_current_file(None);
    context.check_passed(format!("staged_content foreach '{}'", pattern));
    Ok(true)
}

fn execute_allow_command(cmd: &str, context: &mut ExecutionContext) -> Result<bool> {
    context.add_allowed_command(cmd.to_string());
    context.check_passed(format!("allow '{}'", cmd));
    Ok(true)
}

fn execute_staged_files(pattern: &str, body: &[Statement], context: &mut ExecutionContext, hook_args: &[String]) -> Result<bool> {
    let files = context.staged_files(pattern)?;

    if files.is_empty() {
        return Ok(true);
    }

    context.set_file_pattern(Some(pattern.to_string()));

    for file in files {
        context.enter_file(file.clone());

        for stmt in body {
            if !execute_statement(stmt, context, hook_args)? {
                context.leave_file();
                context.set_file_pattern(None);
                return Ok(false);
            }
        }

        context.leave_file();
    }

    context.set_file_pattern(None);
    context.check_passed(format!("staged_files matching '{}'", pattern));
    Ok(true)
}

fn execute_all_files(pattern: &str, body: &[Statement], context: &mut ExecutionContext, hook_args: &[String]) -> Result<bool> {
    let files = githook_git::get_all_files(pattern)?;

    if files.is_empty() {
        return Ok(true);
    }

    context.set_file_pattern(Some(pattern.to_string()));

    for stmt in body {
        if !execute_statement(stmt, context, hook_args)? {
            context.set_file_pattern(None);
            return Ok(false);
        }
    }

    context.set_file_pattern(None);
    Ok(true)
}

fn execute_file_rule(pattern: &str, must_be_staged: bool, context: &mut ExecutionContext) -> Result<bool> {
    let staged = githook_git::is_file_staged(pattern)?;

    if must_be_staged {
        if !staged {
            println!("  {} File matching {} must be staged!", "x".red(), pattern.red());
            return Ok(false);
        }
        context.check_passed(format!("File '{}' must be staged", pattern));
    } else {
        if staged {
            println!("  {} File matching {} must not be staged!", "x".red(), pattern.red());
            return Ok(false);
        }
        context.check_passed(format!("File '{}' must not be staged", pattern));
    }
    Ok(true)
}

fn execute_content_validation(
    scope: &ContentScope,
    must: bool,
    check: &ContentCheck,
    pattern: &Option<String>,
    context: &mut ExecutionContext
) -> Result<bool> {
    context.check_run();

    let context_pattern = context.file_pattern();
    let file_pattern = pattern
        .as_deref()
        .or(context_pattern)
        .unwrap_or("*");

    let content = match scope {
        ContentScope::Content(_) => {
            if let Some(file) = context.current_file() {
                githook_git::get_staged_file_content_from_index(file)?
            } else {
                githook_git::get_staged_file_content(file_pattern)?
            }
        }
        ContentScope::Diff(_) => githook_git::get_diff_added_lines()?,
    };

    let scope_name = match scope {
        ContentScope::Content(_) => "content",
        ContentScope::Diff(_) => "diff",
    };

    let result = match check {
        ContentCheck::Match(pattern, _) => {
            let regex = Regex::new(pattern)?;
            regex.is_match(&content)
        }
        ContentCheck::Contain(text, _) => content.contains(text),
    };

    let passed = if must { result } else { !result };

    if !passed {
        let check_desc = match check {
            ContentCheck::Match(p, _) => format!("match pattern '{}'", p),
            ContentCheck::Contain(t, _) => format!("contain '{}'", t),
        };
        let must_str = if must { "must" } else { "must not" };
        println!("  {} {} {} {}", "x".red(), scope_name.red(), must_str.red(), check_desc.red());
        return Ok(false);
    }

    let check_desc = match check {
        ContentCheck::Match(p, _) => format!(
            "{} must{} match '{}'",
            scope_name,
            if must { "" } else { " not" },
            p
        ),
        ContentCheck::Contain(t, _) => format!(
            "{} must{} contain '{}'",
            scope_name,
            if must { "" } else { " not" },
            t
        ),
    };

    context.check_passed(check_desc);
    Ok(true)
}

fn execute_conditional_rule(
    severity: &RuleSeverity,
    condition: &githook_syntax::BlockCondition,
    message: &Option<String>,
    interactive: &Option<String>,
    context: &mut ExecutionContext,
    hook_args: &[String]
) -> Result<bool> {
    context.check_run();

    let result = evaluate_block_condition(condition, context, hook_args)?;

    let default_message = condition.default_message();
    let raw_message = message.as_deref().unwrap_or(&default_message);
    let message_cow = substitute_placeholders(raw_message, context);
    let message_str = message_cow.as_ref();

    if result {
        match severity {
            RuleSeverity::Warn(_) => {
                println!("  {} {}", "-".yellow(), message_str.yellow());

                if let Some(prompt) = interactive {
                    if atty::is(atty::Stream::Stdin) {
                        let prompt_cow = substitute_placeholders(prompt, context);
                        println!("\n  {} {}", "?".cyan(), prompt_cow.cyan());
                        print!("    Continue? (y/n): ");
                        std::io::Write::flush(&mut std::io::stdout()).ok();

                        let mut input = String::new();
                        match std::io::stdin().read_line(&mut input) {
                            Ok(_) => {
                                let answer = input.trim().to_lowercase();
                                if answer != "y" && answer != "yes" {
                                    println!("  {} Aborted by user", "x".red());
                                    return Ok(false);
                                }
                            }
                            Err(_) => {
                                println!("  {} (Skipping interactive prompt - no TTY)", "!".yellow());
                            }
                        }
                    } else {
                        println!("  {} (Skipping interactive prompt - running in non-interactive mode)", "!".yellow());
                    }
                }

                context.warn(message_str.to_string());
                return Ok(true);
            }
            RuleSeverity::Block(_) => {
                println!("  {} {}", "x".red(), message_str.red());
                context.fail_check(message_str.to_string());
                return Ok(false);
            }
        }
    }

    Ok(true)
}

fn execute_message_validation(must: bool, check: &MessageCheck, context: &mut ExecutionContext, hook_args: &[String]) -> Result<bool> {
    context.check_run();

    let msg = githook_git::get_commit_message_from_hook_args(hook_args)?;
    let result = match check {
        MessageCheck::Match(pattern, _) => {
            let regex = Regex::new(pattern)?;
            regex.is_match(&msg)
        }
        MessageCheck::Contain(text, _) => msg.contains(text),
    };

    let passed = if must { result } else { !result };

    if !passed {
        let check_desc = match check {
            MessageCheck::Match(p, _) => format!("match pattern '{}'", p),
            MessageCheck::Contain(t, _) => format!("contain '{}'", t),
        };
        let must_str = if must { "must" } else { "must not" };
        println!("  {} Commit message {} {}", "x".red(), must_str.red(), check_desc.red());
        println!("    Message: {}", msg.dimmed());
        return Ok(false);
    }

    let check_desc = match check {
        MessageCheck::Match(p, _) => format!(
            "Commit message must{} match '{}'",
            if must { "" } else { " not" },
            p
        ),
        MessageCheck::Contain(t, _) => format!(
            "Commit message must{} contain '{}'",
            if must { "" } else { " not" },
            t
        ),
    };

    context.check_passed(check_desc);
    Ok(true)
}

//
fn execute_macro_definition(name: String, params: Vec<String>, body: Vec<Statement>, context: &mut ExecutionContext) -> Result<bool> {
    context.define_macro(name, params, body);
    Ok(true)
}

//
pub fn execute_macro_call(namespace: Option<&str>, name: &str, args: &[Argument], context: &mut ExecutionContext, hook_args: &[String]) -> Result<bool> {
    let lookup_name = if let Some(ns) = namespace {
        format!("{}:{}", ns, name)
    } else {
        name.to_string()
    };

    let snippet = context.get_macro(&lookup_name)
        .or_else(|| context.get_macro(name))
        .ok_or_else(|| anyhow::anyhow!("Undefined macro @{}", if namespace.is_some() { &lookup_name } else { name }))?
        .clone();

    if args.len() != snippet.params.len() {
        bail!("macro @{} expects {} arguments, got {}", lookup_name, snippet.params.len(), args.len());
    }

    for (param, arg) in snippet.params.iter().zip(args.iter()) {
        let value = match arg {
            Argument::String(s, _) => s.clone(),
            Argument::Number(n, _) => n.to_string(),
            Argument::Identifier(id, _) => id.clone(),
            Argument::Array(_, _) => {
                bail!("Array arguments are not supported in macro calls");
            }
        };
        context.set_param(param.clone(), value);
    }
    
    for statement in &snippet.body {
        if !execute_statement(statement, context, hook_args)? {
            context.clear_params();
            return Ok(false);
        }
    }

    context.clear_params();
    Ok(true)
}

fn execute_use_statement(
    namespace: &str,
    name: &str,
    alias: Option<&str>,
    context: &mut ExecutionContext,
    hook_args: &[String],
) -> Result<bool> {
    use crate::package_resolver;

    let content = package_resolver::load_package(namespace, name)?;

    let tokens = githook_syntax::tokenize_with_spans(&content)?;
    let statements = githook_syntax::parse_spanned(tokens)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let package_ns = alias.unwrap_or(name);

    for statement in statements {
        let scoped_statement = if let githook_syntax::Statement::MacroDefinition { name: macro_name, params, body, span } = &statement {
            githook_syntax::Statement::MacroDefinition {
                name: format!("{}:{}", package_ns, macro_name),
                params: params.clone(),
                body: body.clone(),
                span: *span,
            }
        } else {
            statement
        };

        if !execute_statement(&scoped_statement, context, hook_args)? {
            return Ok(false);
        }
    }

    Ok(true)
}

fn execute_import_statement(
    path: &str,
    alias: Option<&str>,
    context: &mut ExecutionContext,
    hook_args: &[String],
) -> Result<bool> {
    use std::path::Path;

    let file_path = if Path::new(path).is_absolute() {
        path.to_string()
    } else {
        let cwd = std::env::current_dir()?;
        cwd.join(path).to_string_lossy().to_string()
    };

    let content = std::fs::read_to_string(&file_path)?;
    let tokens = githook_syntax::tokenize_with_spans(&content)?;
    let statements = githook_syntax::parse_spanned(tokens)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    for statement in statements {
        let scoped_statement = if let githook_syntax::Statement::MacroDefinition { name: macro_name, params, body, span } = &statement {
            if let Some(alias_ns) = alias {
                githook_syntax::Statement::MacroDefinition {
                    name: format!("{}:{}", alias_ns, macro_name),
                    params: params.clone(),
                    body: body.clone(),
                    span: *span,
                }
            } else {
                statement
            }
        } else {
            statement
        };

        if !execute_statement(&scoped_statement, context, hook_args)? {
            return Ok(false);
        }
    }

    Ok(true)
}

fn execute_when_statement(condition: &githook_syntax::BlockCondition, body: &[Statement], else_body: &Option<Vec<Statement>>, context: &mut ExecutionContext, hook_args: &[String]) -> Result<bool> {
    let result = evaluate_block_condition(condition, context, hook_args)?;

    if result {
        for statement in body {
            if !execute_statement(statement, context, hook_args)? {
                return Ok(false);
            }
        }
    } else if let Some(else_stmts) = else_body {
        for statement in else_stmts {
            if !execute_statement(statement, context, hook_args)? {
                return Ok(false);
            }
        }
    }

    context.check_passed(format!("when condition evaluated to {}", result));
    Ok(true)
}

pub fn execute_statement(
    statement: &Statement,
    context: &mut ExecutionContext,
    hook_args: &[String],
) -> Result<bool> {
    match statement {
        Statement::Run(cmd, _) => execute_run(cmd, context),
        Statement::BoolLiteral(value, _) => execute_bool_literal(*value, context),
        Statement::Group { definition, span: _ } => execute_group(definition, context, hook_args),
        Statement::LetStringList { name, items, .. } => execute_let_string_list(name.clone(), items.clone(), context),
        Statement::Block(msg, _) => execute_block(msg),
        Statement::ForEachStringList { var, list, body, .. } => execute_foreach_string_list(var, list, body, context, hook_args),
        Statement::ForEachArray { var, items, body, .. } => execute_foreach_array(var, items, body, context, hook_args),
        Statement::ForEachStagedFiles { var, pattern, where_cond, body, .. } => {
            execute_foreach_staged_files(var, pattern, where_cond, body, context, hook_args)
        }
        Statement::Parallel { commands, .. } => execute_parallel(commands, context),
        Statement::StagedFiles { pattern, body, .. } => execute_staged_files(pattern, body, context, hook_args),
        Statement::StagedContentValidation { must, check, pattern, .. } => {
            execute_staged_content_validation(*must, check, pattern, context)
        }
        Statement::StagedContentForeach { pattern, body, .. } => execute_staged_content_foreach(pattern, body, context, hook_args),
        Statement::AllowCommand(cmd, _) => execute_allow_command(cmd, context),
        Statement::AllFiles { pattern, body, .. } => execute_all_files(pattern, body, context, hook_args),
        Statement::FileRule { pattern, must_be_staged, .. } => execute_file_rule(pattern, *must_be_staged, context),
        Statement::ContentValidation { scope, must, check, pattern, .. } => {
            execute_content_validation(scope, *must, check, pattern, context)
        }
        Statement::ConditionalRule { severity, condition, message, interactive, .. } => {
            execute_conditional_rule(severity, condition, message, interactive, context, hook_args)
        }
        Statement::MessageValidation { must, check, .. } => execute_message_validation(*must, check, context, hook_args),
        
        Statement::MacroDefinition { name, params, body, span: _ } => {
            execute_macro_definition(name.clone(), params.clone(), body.clone(), context)
        }
        Statement::MacroCall { namespace, name, args, .. } => execute_macro_call(namespace.as_deref(), name, args, context, hook_args),
        Statement::When { condition, body, else_body, span: _ } => {
            execute_when_statement(condition, body, else_body, context, hook_args)
        }
        Statement::Match { subject, arms, span: _ } => {
            execute_match_statement(subject, arms, context, hook_args)
        }
        Statement::Use { namespace, name, alias, .. } => {
            execute_use_statement(namespace, name, alias.as_deref(), context, hook_args)
        }
        Statement::Import { path, alias, .. } => {
            execute_import_statement(path, alias.as_deref(), context, hook_args)
        }
    }
}

fn execute_match_statement(
    subject: &MatchSubject,
    arms: &[MatchArm],
    context: &mut ExecutionContext,
    hook_args: &[String],
) -> Result<bool> {
    let value_to_match = match subject {
        MatchSubject::File(_) => {
            context.current_file.clone().ok_or_else(|| {
                anyhow::anyhow!("'match file' requires a current file (use inside foreach staged_files ...)")
            })?
        }
        MatchSubject::Content(_) => {
            let file = context.current_file.as_ref().ok_or_else(|| {
                anyhow::anyhow!("'match content' requires a current file (use inside foreach staged_files ...)")
            })?;
            std::fs::read_to_string(file)
                .with_context(|| format!("Failed to read file: {}", file))?
        }
        MatchSubject::Diff(_) => {
            let file = context.current_file.as_ref().ok_or_else(|| {
                anyhow::anyhow!("'match diff' requires a current file (use inside foreach staged_files ...)")
            })?;
            githook_git::get_staged_file_content_from_index(file)
                .with_context(|| format!("Failed to get diff for: {}", file))?
        }
    };

    for arm in arms {
        let matches = match &arm.pattern {
            MatchPattern::Wildcard(pattern, _) => {
                if matches!(subject, MatchSubject::File(_)) {
                    match get_cached_glob(pattern) {
                        Ok(glob_pattern) => glob_pattern.matches(&value_to_match),
                        Err(e) => {
                            eprintln!("Warning: Invalid glob pattern '{}': {}", pattern, e);
                            false
                        }
                    }
                } else {
                    false
                }
            }
            MatchPattern::Contains(text, _) => {
                value_to_match.contains(text)
            }
            MatchPattern::Matches(regex_str, _) => {
                let regex = regex::Regex::new(regex_str)
                    .with_context(|| format!("Invalid regex: {}", regex_str))?;
                regex.is_match(&value_to_match)
            }
            MatchPattern::GreaterThan(threshold, _) => {
                if matches!(subject, MatchSubject::File(_)) {
                    let metadata = std::fs::metadata(&value_to_match)?;
                    (metadata.len() as f64) > *threshold
                } else {
                    false
                }
            }
            MatchPattern::LessThan(threshold, _) => {
                if matches!(subject, MatchSubject::File(_)) {
                    let metadata = std::fs::metadata(&value_to_match)?;
                    (metadata.len() as f64) < *threshold
                } else {
                    false
                }
            }
        };

        if matches {
            for stmt in &arm.action {
                if !execute_statement(stmt, context, hook_args)? {
                    return Ok(false);
                }
            }
            return Ok(true);
        }
    }

    Ok(true)
}
