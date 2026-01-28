use crate::context::ExecutionContext;
use crate::executor::execute_macro_call;
use anyhow::Result;
use githook_syntax::BlockCondition;
use githook_syntax::*;
use colored::*;
use regex::Regex;
use std::borrow::Cow;
use std::sync::OnceLock;

static REGEX_CACHE: OnceLock<std::sync::Mutex<lru::LruCache<String, Regex>>> = OnceLock::new();

const DEFAULT_CACHE_SIZE: usize = 100;

fn get_regex_cache_size() -> usize {
    std::env::var("GITHOOK_REGEX_CACHE_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_CACHE_SIZE)
}

fn get_cached_regex(pattern: &str) -> Result<Regex> {
    let cache = REGEX_CACHE.get_or_init(|| {
        let size = get_regex_cache_size();
        std::sync::Mutex::new(
            lru::LruCache::new(
                std::num::NonZeroUsize::new(size)
                    .unwrap_or_else(|| std::num::NonZeroUsize::new(DEFAULT_CACHE_SIZE).unwrap())
            )
        )
    });

    let mut cache = cache.lock()
        .expect("Regex cache mutex should not be poisoned");
    
    if let Some(regex) = cache.get(pattern) {
        return Ok(regex.clone());
    }

    let regex = Regex::new(pattern)?;
    cache.put(pattern.to_string(), regex.clone());
    Ok(regex)
}

fn resolve_value<'a>(name: &'a str, context: &'a ExecutionContext) -> Cow<'a, str> {
    if let Some(v) = context.get_var(name) {
        return Cow::Owned(v.to_string());
    }

    let file = match context.current_file() {
        Some(f) => f,
        None => return Cow::Borrowed(name),
    };
    let path = std::path::Path::new(file);

    match name {
        "file" => Cow::Borrowed(file),
        "extension" => {
            path.extension()
                .and_then(|s| s.to_str())
                .map(|s| Cow::Owned(format!(".{}", s)))
                .unwrap_or(Cow::Borrowed(name))
        }
        "basename" => {
            path.file_name()
                .and_then(|s| s.to_str())
                .map(Cow::Borrowed)
                .unwrap_or(Cow::Borrowed(name))
        }
        "dirname" => {
            path.parent()
                .and_then(|p| p.to_str())
                .map(Cow::Borrowed)
                .unwrap_or(Cow::Borrowed(name))
        }
        _ => Cow::Borrowed(name),
    }
}

fn evaluate_comparison(
    left: &PropertyValue,
    operator: &ComparisonOperator,
    right: &ComparisonValue,
    context: &mut ExecutionContext,
) -> Result<bool> {
    use PropertyValue::*;
    use ComparisonOperator::*;
    
    match (left, operator) {
        (FileSize(..), Greater | GreaterOrEqual | Less | LessOrEqual | Equals) => {
            let file_size = if let Some(file) = context.current_file() {
                githook_git::get_staged_file_size_from_index(file)? as f64
            } else {
                return Ok(false);
            };
            
            let threshold = match right {
                ComparisonValue::Number(n, _) => *n,
                ComparisonValue::Identifier(id, _) => {
                    context.get_param(id)
                        .ok_or_else(|| anyhow::anyhow!("Undefined parameter: {}", id))?
                        .parse::<f64>()
                        .map_err(|_| anyhow::anyhow!("Parameter {} is not a valid number", id))?
                }
                _ => anyhow::bail!("file_size comparison requires numeric value"),
            };
            
            Ok(match operator {
                Greater => file_size > threshold,
                GreaterOrEqual => file_size >= threshold,
                Less => file_size < threshold,
                LessOrEqual => file_size <= threshold,
                Equals => file_size == threshold,
                _ => unreachable!(),
            })
        }
        
        (ModifiedLines(..) | FilesChanged(..) | Additions(..) | Deletions(..) | CommitsAhead(..), 
         Greater | GreaterOrEqual | Less | LessOrEqual | Equals) => {
            let value = match left {
                ModifiedLines(..) => githook_git::get_modified_lines()? as f64,
                FilesChanged(..) => githook_git::get_diff_stats()?.files_changed as f64,
                Additions(..) => githook_git::get_diff_stats()?.additions as f64,
                Deletions(..) => githook_git::get_diff_stats()?.deletions as f64,
                CommitsAhead(..) => {
                    let branch = githook_git::get_branch_name()?;
                    let remote_branch = format!("origin/{}", branch);
                    githook_git::get_commits_ahead(&remote_branch)? as f64
                }
                _ => unreachable!(),
            };
            
            let threshold = match right {
                ComparisonValue::Number(n, _) => *n,
                ComparisonValue::Identifier(id, _) => {
                    // Try to resolve parameter from context
                    context.get_param(id)
                        .ok_or_else(|| anyhow::anyhow!("Undefined parameter: {}", id))?
                        .parse::<f64>()
                        .map_err(|_| anyhow::anyhow!("Parameter {} is not a valid number", id))?
                }
                _ => anyhow::bail!("Numeric property comparison requires numeric value"),
            };
            
            Ok(match operator {
                Greater => value > threshold,
                GreaterOrEqual => value >= threshold,
                Less => value < threshold,
                LessOrEqual => value <= threshold,
                Equals => value == threshold,
                _ => unreachable!(),
            })
        }
        
        (BranchName(..) | Content(..) | Diff(..) | CommitMessage(..), Matches) => {
            let text = match left {
                BranchName(..) => githook_git::get_branch_name()?,
                Content(..) => context.current_file_content()?,
                Diff(..) => context.current_file_diff()?,
                CommitMessage(..) => context.get_commit_message()?,
                _ => unreachable!(),
            };
            
            let pattern = match right {
                ComparisonValue::String(s, _) => s,
                _ => anyhow::bail!("matches operator requires string pattern"),
            };
            
            let regex = get_cached_regex(pattern)?;
            Ok(regex.is_match(&text))
        }
        
        (BranchName(..) | Content(..) | Diff(..) | CommitMessage(..) | Extension(..) | Filename(..) | Basename(..) | Dirname(..) | EnvVar(_, _) | Placeholder(_, _), Equals) => {
            let text = match left {
                BranchName(..) => githook_git::get_branch_name()?,
                Content(..) => context.current_file_content()?,
                Diff(..) => context.current_file_diff()?,
                CommitMessage(..) => context.get_commit_message()?,
                Extension(..) => resolve_value("extension", context).into_owned(),
                Filename(..) => {
                    context.current_file()
                        .and_then(|f| std::path::Path::new(f).file_name())
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string()
                }
                Basename(..) => resolve_value("basename", context).into_owned(),
                Dirname(..) => resolve_value("dirname", context).into_owned(),
                EnvVar(key, _) => {
                    std::env::var(key)
                        .map_err(|_| anyhow::anyhow!("environment variable '{}' not defined", key))?
                }
                Placeholder(placeholder_str, _) => {
                    let parts: Vec<&str> = placeholder_str.split(':').collect();
                    if parts.len() == 2 {
                        context.placeholder_registry()
                            .resolve(parts[0], parts[1], context)
                            .unwrap_or_default()
                    } else {
                        String::new()
                    }
                }
                _ => unreachable!(),
            };

            let needle = match right {
                ComparisonValue::String(s, _) => s,
                ComparisonValue::Identifier(id, _) => id,
                _ => anyhow::bail!("equals operator requires string value"),
            };

            Ok(text == *needle)
        }

        (BranchName(..) | Content(..) | Diff(..) | CommitMessage(..) | Extension(..) | Filename(..) | Basename(..) | Dirname(..) | EnvVar(_, _) | Placeholder(_, _), Contains) => {
            let text = match left {
                BranchName(..) => githook_git::get_branch_name()?,
                Content(..) => context.current_file_content()?,
                Diff(..) => context.current_file_diff()?,
                CommitMessage(..) => context.get_commit_message()?,
                Extension(..) => {
                    resolve_value("extension", context).into_owned()
                }
                Filename(..) => {
                    context.current_file()
                        .and_then(|f| std::path::Path::new(f).file_name())
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string()
                }
                Basename(..) => {
                    resolve_value("basename", context).into_owned()
                }
                Dirname(..) => {
                    resolve_value("dirname", context).into_owned()
                }
                EnvVar(key, _) => {
                    std::env::var(key)
                        .map_err(|_| anyhow::anyhow!("environment variable '{}' not defined", key))?
                }
                Placeholder(placeholder_str, _) => {
                    let parts: Vec<&str> = placeholder_str.split(':').collect();
                    if parts.len() == 2 {
                        context.placeholder_registry()
                            .resolve(parts[0], parts[1], context)
                            .unwrap_or_default()
                    } else {
                        String::new()
                    }
                }
                _ => unreachable!(),
            };
            
            let needle = match right {
                ComparisonValue::String(s, _) => s,
                ComparisonValue::Identifier(id, _) => id,
                _ => anyhow::bail!("contains operator requires string value"),
            };
            
            Ok(text.contains(needle))
        }
        
        (Extension(..), In) => {
            let current = resolve_value("extension", context).into_owned();
            
            let list_name = match right {
                ComparisonValue::ListIdentifier(name, _) => name,
                _ => anyhow::bail!("'in' operator requires list identifier"),
            };
            
            let items = match context.get_string_list(list_name) {
                Some(xs) => xs,
                None => {
                    println!("  {} unknown string list '{}'", "x".red(), list_name.red());
                    return Ok(false);
                }
            };
            
            Ok(items.iter().any(|x| x == &current))
        }
        
        _ => anyhow::bail!("Unsupported comparison: {:?} {:?} {:?}", left, operator, right),
    }
}

pub fn evaluate_block_condition(
    condition: &BlockCondition, 
    context: &mut ExecutionContext, 
    hook_args: &[String]
) -> Result<bool> {
    match condition {
        BlockCondition::Comparison { left, operator, right, negated, .. } => {
            let result = evaluate_comparison(left, operator, right, context)?;
            Ok(if *negated { !result } else { result })
        }
        
        BlockCondition::ContainsSecrets(_) => {
            let findings = githook_git::secrets_with_locations()?;
            if !findings.is_empty() {
                println!("  {} Secrets detected!", "x".red());
                for f in &findings {
                    println!("    {}:{} {}", f.file, f.line, f.line_content.trim());
                }
                return Ok(true);
            }
            Ok(false)
        }

        BlockCondition::ContentCheck { scope, check, .. } => {
            let text = match scope {
                ContentScope::Content(_) => context.current_file_content()?,
                ContentScope::Diff(_) => context.current_file_diff()?,
            };

            let ok = match check {
                ContentCheck::Match(pattern, _) => {
                    let regex = get_cached_regex(pattern)?;
                    regex.is_match(&text)
                }
                ContentCheck::Contain(needle, _) => text.contains(needle),
            };
            Ok(ok)
        }

        BlockCondition::And { left, right, .. } => {
            Ok(evaluate_block_condition(left, context, hook_args)? 
                && evaluate_block_condition(right, context, hook_args)?)
        }

        BlockCondition::Or { left, right, .. } => {
            Ok(evaluate_block_condition(left, context, hook_args)? 
                || evaluate_block_condition(right, context, hook_args)?)
        }

        BlockCondition::StringEquals { left, right, right_is_identifier, .. } => {
            let l = resolve_value(left, context);
            let r = if *right_is_identifier {
                resolve_value(right, context)
            } else {
                Cow::Borrowed(right.as_str())
            };
            Ok(l == r)
        }

        BlockCondition::Not { inner, .. } => {
            Ok(!evaluate_block_condition(inner, context, hook_args)?)
        }

        BlockCondition::Bool(b, _) => Ok(*b),

        BlockCondition::InStringList { value, list, .. } => {
            let current = resolve_value(value, context);

            let items = match context.get_string_list(list) {
                Some(xs) => xs,
                None => {
                    println!("  {} unknown string list '{}'", "x".red(), list.red());
                    return Ok(false);
                }
            };

            Ok(items.iter().any(|x| x == &current))
        }

        BlockCondition::AuthorSet(_) => {
            Ok(githook_git::is_author_set()?)
        }

        BlockCondition::AuthorEmailSet(_) => {
            Ok(githook_git::is_author_email_set()?)
        }

        BlockCondition::AuthorMissing(_) => {
            let author_missing = !githook_git::is_author_set()?;
            let email_missing = !githook_git::is_author_email_set()?;
            Ok(author_missing || email_missing)
        }

        BlockCondition::EnvEquals(key, expected, _) => {
            let actual = std::env::var(key)
                .map_err(|_| anyhow::anyhow!("environment variable '{}' not defined", key))?;
            Ok(actual == *expected)
        }

        BlockCondition::MacroCall { name, args, .. } => {
            execute_macro_call(None, name, args, context, hook_args)
        }

        BlockCondition::NotMacroCall { name, args, .. } => {
            let result = execute_macro_call(None, name, args, context, hook_args)?;
            Ok(!result)
        }
    }
}