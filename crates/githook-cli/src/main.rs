mod updater;
mod config;
mod errors;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;
use githook_eval::{Executor, ExecutionResult};
use githook_syntax::{lexer, parser};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use config::Config;
use errors::{enhance_error, EnhancedError};

#[derive(Parser)]
#[command(name = "githook")]
#[command(about = "Git hook language and executor", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(long)]
    cache: bool,

    #[arg(long)]
    no_cache: bool,

    #[arg(long = "only-group", value_name = "GROUPS")]
    only_groups: Option<String>,

    #[arg(long = "skip-group", value_name = "GROUPS")]
    skip_groups: Option<String>,

    #[arg(value_name = "HOOK_TYPE")]
    hook_type: Option<String>,

    #[arg(trailing_var_arg = true)]
    hook_args: Vec<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// List installed packages
    List,
    /// Check for githook updates
    CheckUpdate,
    /// Update githook to latest version
    Update,
    /// Initialize githook in current repository
    Init {
        /// Create config file with defaults
        #[arg(long)]
        config: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load config file (.githookrc)
    let mut config = Config::load().unwrap_or_default();
    
    // Merge CLI arguments
    let cache_opt = if cli.cache {
        Some(true)
    } else if cli.no_cache {
        Some(false)
    } else {
        None
    };
    
    config.merge_cli_args(
        cache_opt,
        false,
        cli.only_groups.clone(),
        cli.skip_groups.clone(),
    );

    if let Some(command) = cli.command {
        return match command {
            Commands::List => list_packages(),
            Commands::CheckUpdate => updater::check_for_updates(),
            Commands::Update => updater::perform_update(),
            Commands::Init { config: create_config } => {
                if create_config {
                    init_config()
                } else {
                    init_hooks()
                }
            }
        };
    }

    let hook_type = determine_hook_type(cli.hook_type, &cli.hook_args)?;
    
    let config_path = match find_config(&hook_type) {
        Ok(path) => path,
        Err(e) => {
            let enhanced = enhance_error(e, None, None)
                .with_suggestion(format!("Create a hook file: touch .githook/{}.ghook", hook_type))
                .with_help("Hook files should be placed in .githook/ or .git/hooks/ directory");
            enhanced.display();
            std::process::exit(1);
        }
    };

    if config.colored {
        println!("{} {}", "→".cyan().bold(), format!("Running {}...", config_path.display()).bold());
    } else {
        println!("→ Running {}...", config_path.display());
    }

    let source = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config from {:?}", config_path))?;

    // ========================================================================
    // V2 SYSTEM: Lexer → Parser → Executor
    // ========================================================================
    
    // 1. Tokenize
    let tokens = match lexer::tokenize(&source) {
        Ok(tokens) => tokens,
        Err(lex_error) => {
            let span = lex_error.span();
            let enhanced = EnhancedError::new(format!("Syntax error: {}", lex_error))
                .with_span(span)
                .with_file(config_path.display().to_string())
                .with_source(source.clone())
                .with_help("Check for invalid characters or unclosed strings");
            enhanced.display();
            std::process::exit(1);
        }
    };

    // 2. Parse
    let statements = match parser::parse(tokens) {
        Ok(stmts) => stmts,
        Err(parse_error) => {
            let enhanced = EnhancedError::new(format!("Parse error: {}", parse_error))
                .with_file(config_path.display().to_string())
                .with_source(source.clone())
                .with_suggestion("Check that all blocks are properly closed with { and }")
                .with_help("Common issues: missing closing brace, missing semicolon, typo in keyword");
            enhanced.display();
            std::process::exit(1);
        }
    };

    // 3. Get git files
    let git_files = get_git_files(&hook_type)?;
    
    // 4. Execute with V2 executor
    let mut executor = Executor::new()
        .with_git_files(git_files);
    executor.verbose = config.verbose;

    let result = match executor.execute_statements(&statements) {
        Ok(res) => res,
        Err(e) => {
            let enhanced = enhance_error(e, Some(config_path.display().to_string()), Some(source))
                .with_help("Check variable names, function calls, and command syntax");
            enhanced.display();
            std::process::exit(1);
        }
    };

    // Print summary
    println!();
    
    // Print all checks in structured format
    if !executor.check_results.is_empty() {
        for check in &executor.check_results {
            let status_text: colored::ColoredString = match check.status {
                githook_eval::CheckStatus::Passed => "Passed".green(),
                githook_eval::CheckStatus::Skipped => "Skipped".cyan(),
                githook_eval::CheckStatus::Failed => "Failed".red(),
            };
            
            // Severity prefix in color, padded to same width
            let (severity_prefix, prefix_len): (colored::ColoredString, usize) = match check.severity {
                githook_syntax::ast::Severity::Critical => ("[Critical]".red(), 10),
                githook_syntax::ast::Severity::Warning => ("[Warning] ".yellow(), 10),
                githook_syntax::ast::Severity::Info => ("[Info]    ".blue(), 10),
            };
            
            // Format: [Severity] name...................status
            // Calculate padding to align status at column 65
            let base_len = prefix_len + 1 + check.name.len();
            let dots_count = if base_len < 60 { 60 - base_len } else { 1 };
            let dots = ".".repeat(dots_count);
            
            if let Some(reason) = &check.reason {
                println!("{} {}{}{}{}",
                    severity_prefix,
                    check.name,
                    dots,
                    status_text,
                    format!(" ({})", reason).dimmed()
                );
            } else {
                println!("{} {}{}{}",
                    severity_prefix,
                    check.name,
                    dots,
                    status_text
                );
            }
        }
        println!();
    }
    
    if !executor.warnings.is_empty() {
        println!("\n{}", "⚠ Warnings:".yellow().bold());
        for warning in &executor.warnings {
            println!("  {} {}", "!".yellow().bold(), warning);
        }
    }
    
    if !executor.blocks.is_empty() {
        println!("\n{}", "✗ Blocked:".red().bold());
        for block in &executor.blocks {
            println!("  {} {}", "✗".red().bold(), block);
        }
    }
    
    println!();
    match result {
        ExecutionResult::Continue => {
            if executor.tests_run > 0 {
                println!("{} {} {}", 
                    "✓".green().bold(), 
                    "Passed".green().bold(),
                    format!("{} checks", executor.tests_run).dimmed()
                );
            } else {
                println!("{} {}", "✓".green().bold(), "No checks to run".dimmed());
            }
            std::process::exit(0);
        }
        ExecutionResult::Blocked => {
            println!("{} {}", "✗".red().bold(), "Hook blocked".red().bold());
            std::process::exit(1);
        }
        ExecutionResult::Break | ExecutionResult::ContinueLoop => {
            let enhanced = EnhancedError::new("break/continue used outside loop")
                .with_help("break and continue can only be used inside foreach loops");
            enhanced.display();
            std::process::exit(1);
        }
    }
}

fn determine_hook_type(explicit_type: Option<String>, args: &[String]) -> Result<String> {
    if let Some(hook_type) = explicit_type {
        if hook_type.ends_with(".ghook") && Path::new(&hook_type).exists() {
            return Ok(hook_type);
        }
        if is_valid_hook_type(&hook_type) {
            return Ok(hook_type);
        }
    }

    for arg in args {
        if arg.ends_with(".ghook") && Path::new(arg).exists() {
            return Ok(arg.clone());
        }
    }

    if let Ok(current_exe) = std::env::current_exe()
        && let Some(file_name) = current_exe.file_name() {
            let name = file_name.to_string_lossy();

            if name.contains("pre-commit") {
                return Ok("pre-commit".to_string());
            } else if name.contains("commit-msg") {
                return Ok("commit-msg".to_string());
            } else if name.contains("pre-push") {
                return Ok("pre-push".to_string());
            } else if name.contains("post-commit") {
                return Ok("post-commit".to_string());
            }
        }

    Ok("pre-commit".to_string())
}

fn is_valid_hook_type(hook: &str) -> bool {
    matches!(
        hook,
        "pre-commit"
            | "commit-msg"
            | "pre-push"
            | "post-commit"
            | "prepare-commit-msg"
            | "post-checkout"
            | "post-merge"
            | "pre-rebase"
    )
}

fn find_config(hook_type: &str) -> Result<PathBuf> {
    if hook_type.ends_with(".ghook") {
        let path = PathBuf::from(hook_type);
        if path.exists() {
            return Ok(path);
        } else {
            anyhow::bail!("Specified config file does not exist: {}", hook_type);
        }
    }
    
    let locations = vec![
        PathBuf::from(format!(".githook/{}.ghook", hook_type)),
        PathBuf::from(format!(".git/hooks/{}.ghook", hook_type)),
        PathBuf::from(format!("{}.ghook", hook_type)),
    ];
    
    for path in locations {
        if path.exists() {
            return Ok(path);
        }
    }
    
    anyhow::bail!(
        "Could not find config file for hook '{}'. Looked in:\n  - .githook/{}.ghook\n  - .git/hooks/{}.ghook\n  - {}.ghook",
        hook_type,
        hook_type,
        hook_type,
        hook_type
    )
}

fn list_packages() -> Result<()> {
    let home = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
    
    let local_dir = home.join(".githook").join("packages");
    let cache_dir = dirs::cache_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine cache directory"))?
        .join("githook")
        .join("packages");
    
    println!("{}", "Installed Packages:".cyan().bold());
    println!();
    
    let mut found_any = false;
    
    if local_dir.exists() {
        println!("{}", "Local (@local):".green());
        if let Ok(namespaces) = fs::read_dir(&local_dir) {
            for namespace_entry in namespaces.flatten() {
                let _namespace_name = namespace_entry.file_name();
                let namespace_path = namespace_entry.path();
                
                if namespace_path.is_dir()
                    && let Ok(packages) = fs::read_dir(&namespace_path)
                {
                    for package_entry in packages.flatten() {
                        let package_name = package_entry.file_name();
                        let package_path = package_entry.path();
                        
                        if package_path.is_dir() {
                            let ghook_file = package_path.join(format!("{}.ghook", package_name.to_string_lossy()));
                            if ghook_file.exists() {
                                println!("  {} @local/{}", "o".green(), package_name.to_string_lossy());
                                found_any = true;
                            }
                        }
                    }
                }
            }
        }
        println!();
    }
    
    if cache_dir.exists() {
        println!("{}", "Cached (remote):".yellow());
        if let Ok(namespaces) = fs::read_dir(&cache_dir) {
            for namespace_entry in namespaces.flatten() {
                let _namespace_name = namespace_entry.file_name();
                let namespace_path = namespace_entry.path();
                
                if namespace_path.is_dir() {
                    let namespace_display = namespace_path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown");
                    
                    if let Ok(packages) = fs::read_dir(&namespace_path) {
                        for package_entry in packages.flatten() {
                            let package_name = package_entry.file_name();
                            let package_path = package_entry.path();
                            
                            if package_path.is_dir() {
                                let ghook_file = package_path.join(format!("{}.ghook", package_name.to_string_lossy()));
                                if ghook_file.exists() {
                                    println!("  {} @{}/{}", "o".yellow(), namespace_display, package_name.to_string_lossy());
                                    found_any = true;
                                }
                            }
                        }
                    }
                }
            }
        }
        println!();
    }
    
    if !found_any {
        println!("{}", "No packages installed yet.".dimmed());
        println!();
        println!("Install packages by using them in your .ghook files:");
        println!("  {}", "use \"@preview/quality\"".dimmed());
    }
    
    Ok(())
}

fn get_git_files(hook_type: &str) -> Result<Vec<String>> {
    // For pre-commit, get staged files
    if hook_type == "pre-commit" {
        let output = Command::new("git")
            .args(["diff", "--cached", "--name-only", "--diff-filter=ACM"])
            .output()
            .context("Failed to get git staged files")?;
        
        if !output.status.success() {
            return Ok(Vec::new());
        }
        
        let files = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| !line.is_empty())
            .map(|s| s.to_string())
            .collect();
        
        return Ok(files);
    }
    
    // For other hooks, get all tracked files
    let output = Command::new("git")
        .args(["ls-files"])
        .output()
        .context("Failed to get git files")?;
    
    if !output.status.success() {
        return Ok(Vec::new());
    }
    
    let files = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| !line.is_empty())
        .map(|s| s.to_string())
        .collect();
    
    Ok(files)
}

#[allow(dead_code)]
fn validate_config(_statements: &[githook_syntax::ast::Statement], _config_path: &Path) -> Result<()> {
    // V2: Simplified validation - parser already handles most errors
    // TODO: Add semantic validation for groups, etc.
    Ok(())
}

fn init_config() -> Result<()> {
    let config_path = PathBuf::from(".githookrc");
    
    if config_path.exists() {
        println!("{} Config file already exists at {}", "✓".green().bold(), config_path.display());
        return Ok(());
    }
    
    let default_config = r#"# Githook Configuration
# See https://github.com/yourusername/githook for more info

# Enable colored output
colored = true

# Enable verbose output
verbose = false

# Enable package caching
cache = true

# Additional search paths for packages
# search_paths = ["./custom-packages"]

# Environment variables
# [env]
# NODE_ENV = "production"

# Execution timeout in seconds
# timeout = 30
"#;
    
    fs::write(&config_path, default_config)
        .with_context(|| format!("Failed to write config to {:?}", config_path))?;
    
    println!("{} Created config file: {}", "✓".green().bold(), config_path.display());
    Ok(())
}

fn init_hooks() -> Result<()> {
    let hooks_dir = PathBuf::from(".githook");
    
    if !hooks_dir.exists() {
        fs::create_dir(&hooks_dir)
            .with_context(|| format!("Failed to create directory {:?}", hooks_dir))?;
        println!("{} Created directory: {}", "✓".green().bold(), hooks_dir.display());
    }
    
    // Create example pre-commit hook
    let pre_commit_path = hooks_dir.join("pre-commit.ghook");
    if !pre_commit_path.exists() {
        let example_hook = r#"# Example pre-commit hook
# Runs on every commit

group "format-check" {
    on "**/*.rs" {
        run "cargo fmt --check"
    }
}

group "lint" {
    on "**/*.rs" {
        run "cargo clippy -- -D warnings"
    }
}
"#;
        fs::write(&pre_commit_path, example_hook)
            .with_context(|| format!("Failed to write hook to {:?}", pre_commit_path))?;
        println!("{} Created example hook: {}", "✓".green().bold(), pre_commit_path.display());
    }
    
    println!("\n{} Githook initialized!", "✓".green().bold());
    println!("  Edit {} to customize your hooks", pre_commit_path.display());
    println!("  Run {} to create a config file", "githook init --config".cyan());
    
    Ok(())
}