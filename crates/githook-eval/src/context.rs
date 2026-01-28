use anyhow::{Context, Result, anyhow, bail};
use githook_syntax::{Statement, tokenize_with_spans};
use std::collections::HashMap;
use std::sync::Arc;
use crate::stdlib;

const MAX_WARNINGS: usize = 1000;
const MAX_WARNINGS_PER_FILE: usize = 100;
const MAX_CHECKS_PASSED: usize = 10000;

type PlaceholderResolver = Box<dyn Fn(&ExecutionContext) -> Option<String> + Send + Sync>;
type NamespaceRegistry = HashMap<String, PlaceholderResolver>;

#[derive(Debug, Clone)]
pub struct MacroDefinition {
    pub params: Vec<String>,
    pub body: Vec<Statement>,
}

pub struct PlaceholderRegistry {
    namespaces: HashMap<String, NamespaceRegistry>,
}

impl Default for PlaceholderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PlaceholderRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            namespaces: HashMap::with_capacity(8),
        };
        registry.register_defaults();
        registry
    }

    fn register_defaults(&mut self) {
        let mut file_ns = HashMap::with_capacity(6);
        file_ns.insert("path".to_string(), Box::new(|ctx: &ExecutionContext| {
            ctx.current_file().map(|s| s.to_string())
        }) as PlaceholderResolver);
        
        file_ns.insert("name".to_string(), Box::new(|ctx: &ExecutionContext| {
            ctx.current_file()
                .and_then(|f| std::path::Path::new(f).file_name())
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
        }) as PlaceholderResolver);
        
        file_ns.insert("stem".to_string(), Box::new(|ctx: &ExecutionContext| {
            ctx.current_file()
                .and_then(|f| std::path::Path::new(f).file_stem())
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
        }) as PlaceholderResolver);
        
        file_ns.insert("ext".to_string(), Box::new(|ctx: &ExecutionContext| {
            ctx.current_file()
                .and_then(|f| std::path::Path::new(f).extension())
                .and_then(|s| s.to_str())
                .map(|s| format!(".{}", s))
        }) as PlaceholderResolver);
        
        file_ns.insert("dir".to_string(), Box::new(|ctx: &ExecutionContext| {
            ctx.current_file()
                .and_then(|f| std::path::Path::new(f).parent())
                .and_then(|p| p.to_str())
                .map(|s| if s.is_empty() { "." } else { s })
                .map(|s| s.to_string())
        }) as PlaceholderResolver);
        
        file_ns.insert("size".to_string(), Box::new(|ctx: &ExecutionContext| {
            ctx.get_file_size().map(|s| s.to_string())
        }) as PlaceholderResolver);
        
        file_ns.insert("size_kb".to_string(), Box::new(|ctx: &ExecutionContext| {
            ctx.get_file_size().map(|s| format!("{:.2}", s as f64 / 1024.0))
        }) as Box<dyn Fn(&ExecutionContext) -> Option<String> + Send + Sync>);
        
        file_ns.insert("size_mb".to_string(), Box::new(|ctx: &ExecutionContext| {
            ctx.get_file_size().map(|s| format!("{:.2}", s as f64 / 1024.0 / 1024.0))
        }) as PlaceholderResolver);
        
        self.namespaces.insert("file".to_string(), file_ns);
        
        let mut git_ns = HashMap::new();
        git_ns.insert("branch".to_string(), Box::new(|_ctx: &ExecutionContext| {
            githook_git::get_branch_name().ok()
        }) as PlaceholderResolver);

        git_ns.insert("author".to_string(), Box::new(|_ctx: &ExecutionContext| {
            githook_git::get_author_name().ok()
        }) as PlaceholderResolver);

        git_ns.insert("email".to_string(), Box::new(|_ctx: &ExecutionContext| {
            githook_git::get_author_email().ok()
        }) as PlaceholderResolver);
        
        self.namespaces.insert("git".to_string(), git_ns);
        
        let mut commit_ns = HashMap::new();
        commit_ns.insert("message".to_string(), Box::new(|_ctx: &ExecutionContext| {
            githook_git::get_commit_message().ok()
        }) as PlaceholderResolver);
        commit_ns.insert("subject".to_string(), Box::new(|_ctx: &ExecutionContext| {
            if let Ok(msg) = githook_git::get_commit_message() {
                Some(msg.lines().next().unwrap_or("").trim().to_string())
            } else { None }
        }) as PlaceholderResolver);
        commit_ns.insert("subject_len".to_string(), Box::new(|_ctx: &ExecutionContext| {
            if let Ok(msg) = githook_git::get_commit_message() {
                let len = msg.lines().next().unwrap_or("").trim().len();
                Some(len.to_string())
            } else { None }
        }) as PlaceholderResolver);
        commit_ns.insert("lines".to_string(), Box::new(|_ctx: &ExecutionContext| {
            if let Ok(msg) = githook_git::get_commit_message() {
                Some(msg.lines().count().to_string())
            } else { None }
        }) as PlaceholderResolver);
        commit_ns.insert("has_coauthor".to_string(), Box::new(|_ctx: &ExecutionContext| {
            if let Ok(msg) = githook_git::get_commit_message() {
                Some((msg.contains("Co-authored-by:")).to_string())
            } else { None }
        }) as PlaceholderResolver);
        self.namespaces.insert("commit".to_string(), commit_ns);

        let mut repo_ns = HashMap::new();
        repo_ns.insert("root".to_string(), Box::new(|_ctx: &ExecutionContext| {
            githook_git::get_repo_root().ok()
        }) as PlaceholderResolver);
        repo_ns.insert("name".to_string(), Box::new(|_ctx: &ExecutionContext| {
            if let Ok(root) = githook_git::get_repo_root() {
                std::path::Path::new(&root)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string())
            } else { None }
        }) as PlaceholderResolver);
        repo_ns.insert("remote".to_string(), Box::new(|_ctx: &ExecutionContext| {
            githook_git::get_remote_url().ok()
        }) as PlaceholderResolver);
        self.namespaces.insert("repo".to_string(), repo_ns);

        let mut system_ns = HashMap::new();
        system_ns.insert("os".to_string(), Box::new(|_ctx: &ExecutionContext| {
            Some(std::env::consts::OS.to_string())
        }) as PlaceholderResolver);
        system_ns.insert("arch".to_string(), Box::new(|_ctx: &ExecutionContext| {
            Some(std::env::consts::ARCH.to_string())
        }) as PlaceholderResolver);
        system_ns.insert("user".to_string(), Box::new(|_ctx: &ExecutionContext| {
            std::env::var("USER").ok()
        }) as PlaceholderResolver);
        system_ns.insert("home".to_string(), Box::new(|_ctx: &ExecutionContext| {
            std::env::var("HOME").ok()
        }) as PlaceholderResolver);
        self.namespaces.insert("system".to_string(), system_ns);

        let mut diff_ns = HashMap::new();
        diff_ns.insert("additions".to_string(), Box::new(|_ctx: &ExecutionContext| {
            match githook_git::get_diff_stats() {
                Ok(stats) => Some(stats.additions.to_string()),
                Err(_) => None,
            }
        }) as PlaceholderResolver);

        diff_ns.insert("deletions".to_string(), Box::new(|_ctx: &ExecutionContext| {
            match githook_git::get_diff_stats() {
                Ok(stats) => Some(stats.deletions.to_string()),
                Err(_) => None,
            }
        }) as PlaceholderResolver);

        diff_ns.insert("files".to_string(), Box::new(|_ctx: &ExecutionContext| {
            match githook_git::get_diff_stats() {
                Ok(stats) => Some(stats.files_changed.to_string()),
                Err(_) => None,
            }
        }) as PlaceholderResolver);
        diff_ns.insert("modified_lines".to_string(), Box::new(|_ctx: &ExecutionContext| {
            githook_git::get_modified_lines().ok().map(|n| n.to_string())
        }) as PlaceholderResolver);
        self.namespaces.insert("diff".to_string(), diff_ns);

        use std::time::{SystemTime, UNIX_EPOCH};
        let mut time_ns = HashMap::new();
        time_ns.insert("epoch".to_string(), Box::new(|_ctx: &ExecutionContext| {
            match SystemTime::now().duration_since(UNIX_EPOCH) {
                Ok(d) => Some(d.as_secs().to_string()),
                Err(_) => None,
            }
        }) as PlaceholderResolver);
        time_ns.insert("millis".to_string(), Box::new(|_ctx: &ExecutionContext| {
            match SystemTime::now().duration_since(UNIX_EPOCH) {
                Ok(d) => Some(d.as_millis().to_string()),
                Err(_) => None,
            }
        }) as PlaceholderResolver);
        self.namespaces.insert("time".to_string(), time_ns);

        if let Some(file_ns) = self.namespaces.get_mut("file") {
            file_ns.insert("oid".to_string(), Box::new(|ctx: &ExecutionContext| {
                ctx.current_file()
                    .and_then(|f| githook_git::get_staged_blob_oid(f).ok())
            }) as PlaceholderResolver);
        }
    }

    pub fn resolve(&self, namespace: &str, key: &str, context: &ExecutionContext) -> Option<String> {
        if namespace == "env" {
            return std::env::var(key).ok();
        }

        self.namespaces
            .get(namespace)
            .and_then(|ns| ns.get(key))
            .and_then(|resolver| resolver(context))
    }
}

pub struct ExecutionContext {
    warnings: HashMap<String, Vec<String>>,
    checks_passed: Vec<String>,
    checks_failed: Vec<String>,
    checks_run: usize,
    current_file_pattern: Option<String>,
    pub current_file: Option<String>,
    allowed_commands: Vec<String>,
    macros: HashMap<String, MacroDefinition>,
    macro_params: HashMap<String, String>,
    std_macros: HashMap<String, MacroDefinition>,
    string_lists: HashMap<String, Vec<String>>,
    vars: HashMap<String, String>,
    current_file_diff_cache: Option<Arc<String>>,
    current_file_content_cache: Option<Arc<String>>,
    commit_message_cache: Option<Arc<String>>,
    staged_files_cache: Option<Vec<String>>,
    allowed_groups: Option<Vec<String>>,
    skipped_groups: Option<Vec<String>>,
    placeholder_registry: PlaceholderRegistry,
}

impl ExecutionContext {
    pub fn new() -> Self {
        Self::new_with_filters(None, None)
    }
    
    pub fn new_with_filters(allowed_groups: Option<Vec<String>>, skipped_groups: Option<Vec<String>>) -> Self {
        let mut context = Self {
            warnings: HashMap::new(),
            checks_passed: Vec::new(),
            checks_failed: Vec::new(),
            checks_run: 0,
            current_file_pattern: None,
            current_file: None,
            current_file_diff_cache: None,
            current_file_content_cache: None,
            commit_message_cache: None,
            staged_files_cache: None,
            allowed_commands: Vec::new(),
            macros: HashMap::new(),
            placeholder_registry: PlaceholderRegistry::new(),
            macro_params: HashMap::new(),
            std_macros: HashMap::new(),
            string_lists: HashMap::new(),
            vars: HashMap::new(),
            allowed_groups,
            skipped_groups,
        };
        context.load_stdlib()
            .expect("Standard library must load successfully");
        context
    }
    pub fn current_file_path(&self) -> Result<&str> {
        self.current_file
            .as_deref()
            .ok_or_else(|| anyhow!("'content'/'diff' requires a current file (use inside foreach staged_files ...)"))
    }

    pub fn leave_file(&mut self) {
        self.current_file = None;
        self.current_file_content_cache = None;
        self.current_file_diff_cache = None;
    }

    pub fn enter_file(&mut self, file: String) {
        if self.current_file.as_ref() != Some(&file) {
            self.current_file_content_cache = None;
            self.current_file_diff_cache = None;
        }
        self.current_file = Some(file);
    }

    pub fn current_file_content(&mut self) -> Result<String> {
        if let Some(cached) = &self.current_file_content_cache {
            return Ok((**cached).clone());
        }

        let path = self.current_file_path()?;
        let spec = format!(":{}", path);
        let out = githook_git::git_capture(&["show", &spec])?;

        self.current_file_content_cache = Some(Arc::new(out.clone()));
        Ok(out)
    }

    pub fn current_file_diff(&mut self) -> Result<String> {
        if let Some(cached) = &self.current_file_diff_cache {
            return Ok((**cached).clone());
        }

        let path = self.current_file_path()?;
        let out = githook_git::git_capture(&["diff", "--cached", "--", path])?;
        
        self.current_file_diff_cache = Some(Arc::new(out.clone()));
        Ok(out)
    }

    pub fn get_commit_message(&mut self) -> Result<String> {
        if let Some(cached) = &self.commit_message_cache {
            return Ok((**cached).clone());
        }

        let msg = githook_git::get_commit_message()?;
        self.commit_message_cache = Some(Arc::new(msg.clone()));
        Ok(msg)
    }

    pub fn define_macro(&mut self, name: String, params: Vec<String>, body: Vec<Statement>) {
        self.macros.insert(name, MacroDefinition { params, body });
    }

    pub fn get_file_size(&self) -> Option<usize> {
        let file = self.current_file.as_ref()?;
        githook_git::get_staged_file_size_from_index(file).ok()
    }

    pub fn get_macro(&self, name: &str) -> Option<&MacroDefinition> {
        self.macros.get(name)
            .or_else(|| self.std_macros.get(name))
    }

    pub fn set_param(&mut self, name: String, value: String) {
        self.macro_params.insert(name, value);
    }

    pub fn get_param(&self, name: &str) -> Option<String> {
        self.macro_params.get(name).cloned()
    }

    pub fn clear_params(&mut self) {
        self.macro_params.clear();
    }

    pub fn warn(&mut self, msg: String) {
        if self.warnings.len() >= MAX_WARNINGS {
            eprintln!("Warning: Maximum warning limit ({}) reached, dropping new warnings", MAX_WARNINGS);
            return;
        }
        
        if let Some(file) = &self.current_file {
            let locations = self.warnings
                .entry(msg)
                .or_default();
            
            if locations.len() < MAX_WARNINGS_PER_FILE {
                locations.push(file.clone());
            }
        } else {
            self.warnings.entry(msg).or_default();
        }
    }

    pub fn check_passed(&mut self, msg: String) {
        if self.checks_passed.len() < MAX_CHECKS_PASSED {
            self.checks_passed.push(msg);
        }
    }

    pub fn check_run(&mut self) {
        self.checks_run += 1;
    }

    pub fn set_var(&mut self, name: String, value: String) {
        self.vars.insert(name, value);
    }

    pub fn staged_files(&mut self, pattern: &str) -> Result<Vec<String>> {
        if pattern == "*" {
            if let Some(files) = &self.staged_files_cache {
                return Ok(files.clone());
            }
            let files = githook_git::get_staged_files(pattern)?;
            self.staged_files_cache = Some(files.clone());
            return Ok(files);
        }

        githook_git::get_staged_files(pattern)
    }

    pub fn get_var(&self, name: &str) -> Option<&str> {
        self.vars.get(name).map(|s| s.as_str())
    }

    pub fn unset_var(&mut self, name: &str) {
        self.vars.remove(name);
    }

    pub fn vars(&self) -> &HashMap<String, String> {
        &self.vars
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    pub fn warnings(&self) -> &HashMap<String, Vec<String>> {
        &self.warnings
    }

    pub fn checks_passed(&self) -> &[String] {
        &self.checks_passed
    }

    pub fn checks_run(&self) -> usize {
        self.checks_run
    }

    pub fn checks_failed(&self) -> &[String] {
        &self.checks_failed
    }

    pub fn has_checks_failed(&self) -> bool {
        !self.checks_failed.is_empty()
    }

    pub fn fail_check(&mut self, desc: String) {
        self.checks_failed.push(desc);
    }

    pub fn set_file_pattern(&mut self, pattern: Option<String>) {
        self.current_file_pattern = pattern;
    }

    pub fn file_pattern(&self) -> Option<&str> {
        self.current_file_pattern.as_deref()
    }

    pub fn set_current_file(&mut self, file: Option<String>) {
        if file != self.current_file {
            self.current_file_content_cache = None;
            self.current_file_diff_cache = None;
        }
        self.current_file = file;
    }

    pub fn current_file(&self) -> Option<&str> {
        self.current_file.as_deref()
    }

    pub fn add_allowed_command(&mut self, cmd: String) {
        self.allowed_commands.push(cmd);
    }

    pub fn is_command_allowed(&self, cmd: &str) -> bool {
        if self.allowed_commands.is_empty() {
            return true;
        }

        let prog = cmd.split_whitespace().next().unwrap_or("");
        self.allowed_commands.iter().any(|c| c == prog)
    }

    pub fn set_string_list(&mut self, name: String, items: Vec<String>) {
        self.string_lists.insert(name, items);
    }

    pub fn get_string_list(&self, name: &str) -> Option<&[String]> {
        self.string_lists.get(name).map(|v| v.as_slice())
    }

    fn load_stdlib(&mut self) -> Result<()> {
        for (module, source) in stdlib::ALL.iter() {
            let source = source.trim();
            if source.is_empty() {
                continue;
            }
            
            let tokens = githook_syntax::tokenize_with_spans(source)
                .with_context(|| format!("Tokenize error in stdlib/{}", module))?;
            
            let statements = match githook_syntax::parse_spanned(tokens) {
                Ok(s) => s,
                Err(e) => {
                    return Err(annotate_parse_error(module, source, anyhow::anyhow!("{}", e)));
                }
            };
            
            for statement in statements {
                if let Statement::MacroDefinition { name, params, body, span: _ } = statement {
                    let fq = format!("std::{}::{}", module, name);
                    
                    if self.std_macros.contains_key(&fq) {
                        bail!("Duplicate stdlib macro: '{}'", fq);
                    }
                    
                    let macro_def = MacroDefinition { 
                        params, 
                        body 
                    };
                    
                    self.std_macros.insert(fq, macro_def.clone());
                    
                    if self.std_macros.contains_key(&name) {
                        bail!("Duplicate macro name: '{}'", name);
                    }
                    
                    self.std_macros.insert(name, macro_def);
                }
            }
        }
        
        Ok(())
    }
    
    pub fn allowed_groups(&self) -> &Option<Vec<String>> {
        &self.allowed_groups
    }
    
    pub fn skipped_groups(&self) -> &Option<Vec<String>> {
        &self.skipped_groups
    }
    
    pub fn placeholder_registry(&self) -> &PlaceholderRegistry {
        &self.placeholder_registry
    }
}

fn annotate_parse_error(module: &str, source: &str, err: anyhow::Error) -> anyhow::Error {
    let msg = err.to_string();

    if let Ok(tokens) = tokenize_with_spans(source) {
        for spanned in tokens {
            let repr = format!("{:?}", spanned.token);
            if msg.contains(&repr) {
                return anyhow!(
                    "Parse error in stdlib/{} at line {}, col {}: {}",
                    module,
                    spanned.span.line,
                    spanned.span.col,
                    msg
                );
            }
        }
    }

    anyhow!("Parse error in stdlib/{}: {}", module, msg)
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self::new()
    }
}