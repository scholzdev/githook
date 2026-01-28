mod updater;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;
use githook::prelude::*;
use githook::{parse_spanned, tokenize_with_spans, Diagnostic};
use std::fs;
use std::path::{Path, PathBuf};

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
    List,
    CheckUpdate,
    Update,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(command) = cli.command {
        return match command {
            Commands::List => list_packages(),
            Commands::CheckUpdate => updater::check_for_updates(),
            Commands::Update => updater::perform_update(),
        };
    }

    let only_groups = cli.only_groups.map(|s| {
        s.split(',').map(|g| g.trim().to_string()).collect()
    });
    
    let skip_groups = cli.skip_groups.map(|s| {
        s.split(',').map(|g| g.trim().to_string()).collect()
    });

    let use_cache = !cli.no_cache && (cli.cache || !cli.no_cache);

    let hook_type = determine_hook_type(cli.hook_type, &cli.hook_args)?;

    let config_path = find_config(&hook_type)?;

    if !use_cache {
        println!(
            "{} Running {} (cache disabled)...",
            "-".cyan(),
            config_path.display()
        );
    } else {
        println!("{} Running {}...", "-".cyan(), config_path.display());
    }

    let source = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config from {:?}", config_path))?;

    let tokens = match tokenize_with_spans(&source) {
        Ok(tokens) => tokens,
        Err(lex_error) => {
            let diagnostic = Diagnostic::new_lex(&source, lex_error);
            eprintln!("{}", diagnostic);
            std::process::exit(1);
        }
    };

    // Parse with span information
    let ast = match parse_spanned(tokens) {
        Ok(ast) => ast.to_vec(),
        Err(parse_error) => {
            let diagnostic = Diagnostic::new_parse(&source, parse_error);
            eprintln!("{}", diagnostic);
            eprintln!(
                "\n{}: Make sure all blocks are properly closed with '{{' and '}}'\n",
                "Tip".yellow().bold()
            );
            std::process::exit(1);
        }
    };

    validate_config(&ast, &config_path)?;

    let status = execute_with_filters(ast, &cli.hook_args, only_groups, skip_groups)
        .with_context(|| "Failed to execute hook")?;

    match status {
        ExecutionStatus::Ok => {
            println!("{} Hook passed!", "✓".green());
            std::process::exit(0);
        }
        ExecutionStatus::Warn => {
            println!("{} Hook passed with warnings", "!".yellow());
            std::process::exit(0);
        }
        ExecutionStatus::Block => {
            println!("{} Hook blocked!", "✗".red());
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

    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(file_name) = current_exe.file_name() {
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

fn validate_config(ast: &[Statement], _config_path: &Path) -> Result<()> {
    let mut warnings = Vec::new();
    
    for (i, stmt) in ast.iter().enumerate() {
        if let Statement::Group { definition, .. } = stmt {
            if definition.name.is_empty() {
                warnings.push(format!("Line {}: Group has empty name", i + 1));
            }
            if definition.name.contains(' ') {
                warnings.push(format!("Line {}: Group name '{}' contains spaces (use hyphens instead)", i + 1, definition.name));
            }
        }
    }
    
    if !warnings.is_empty() {
        eprintln!("{}", "Configuration warnings:".yellow().bold());
        for warning in warnings {
            eprintln!("  {} {}", "!".yellow(), warning);
        }
        eprintln!();
    }
    
    Ok(())
}