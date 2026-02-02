//! # GitHook Git Operations
//!
//! Git integration and operations for the GitHook DSL.
//!
//! ## Overview
//!
//! This crate provides comprehensive Git operations used by GitHook scripts:
//!
//! - **File Operations**: Get staged, unstaged, and all tracked files
//! - **Diff Operations**: Retrieve and parse Git diffs with caching
//! - **Commit Information**: Access commit messages, authors, and metadata
//! - **Branch Information**: Current branch, remote tracking, and branch list
//! - **Secret Detection**: Scan for potential secrets in staged changes
//! - **Merge/Conflict Detection**: Check for merge states and conflicts
//!
//! ## Performance
//!
//! - **LRU Caching**: Diffs and commit messages are cached for performance
//! - **Glob Pattern Caching**: Regex patterns are compiled once and reused
//! - **Streaming Support**: Large diffs can be streamed to avoid memory issues
//!
//! ## Example
//!
//! ```rust,no_run
//! use githook_git::{get_staged_files, get_diff, find_secrets};
//!
//! // Get all staged files
//! let staged = get_staged_files().unwrap();
//! for file in staged {
//!     println!("Staged: {}", file);
//! }
//!
//! // Get diff for a specific file
//! let diff = get_diff("src/main.rs").unwrap();
//! println!("Diff:\n{}", diff);
//!
//! // Scan for potential secrets
//! let secrets = find_secrets().unwrap();
//! if !secrets.is_empty() {
//!     eprintln!("Warning: Potential secrets found!");
//! }
//! ```
//!
//! ## Caching Strategy
//!
//! - **Diff Cache**: 50 entries (LRU)
//! - **Commit Message Cache**: 100 entries (LRU)
//! - **Glob Regex Cache**: Unlimited (compiled patterns)
//!
//! ## Secret Detection
//!
//! The secret scanner looks for common patterns:
//! - API keys and tokens
//! - Private keys (RSA, SSH, etc.)
//! - Passwords and credentials
//! - AWS access keys
//! - Generic secret patterns

use anyhow::{Context, Result, bail};
use lru::LruCache;
use regex::Regex;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::process::Command;
use std::sync::{Mutex, OnceLock};

/// Statistics about a Git diff.
///
/// Contains counts for files changed, additions, and deletions.
#[derive(Debug, Default, Clone)]
pub struct DiffStats {
    /// Number of files changed in the diff
    pub files_changed: usize,
    /// Number of lines added
    pub additions: usize,
    /// Number of lines deleted
    pub deletions: usize,
}

static GLOB_REGEX_CACHE: OnceLock<Mutex<HashMap<String, Regex>>> = OnceLock::new();

static DIFF_CACHE: OnceLock<Mutex<LruCache<String, String>>> = OnceLock::new();
static COMMIT_MSG_CACHE: OnceLock<Mutex<LruCache<String, String>>> = OnceLock::new();

fn get_diff_cache() -> &'static Mutex<LruCache<String, String>> {
    DIFF_CACHE.get_or_init(|| {
        Mutex::new(LruCache::new(
            NonZeroUsize::new(50).expect("Valid cache size"),
        ))
    })
}

fn get_commit_msg_cache() -> &'static Mutex<LruCache<String, String>> {
    COMMIT_MSG_CACHE.get_or_init(|| {
        Mutex::new(LruCache::new(
            NonZeroUsize::new(100).expect("Valid cache size"),
        ))
    })
}

/// A potential secret found in the codebase.
///
/// Contains information about where the secret was found and what the
/// line of code contains.
#[derive(Debug)]
pub struct SecretFinding {
    /// Path to the file containing the potential secret
    pub file: String,
    /// Line number where the secret was found (1-indexed)
    pub line: usize,
    /// The actual line content (may contain sensitive data)
    pub line_content: String,
}

/// Get the size of a staged file from the Git index.
///
/// # Arguments
///
/// * `path` - Relative path to the file in the repository
///
/// # Returns
///
/// Size in bytes, or an error if the file is not staged or doesn't exist.
///
/// # Example
///
/// ```rust,no_run
/// use githook_git::get_staged_file_size_from_index;
///
/// let size = get_staged_file_size_from_index("src/main.rs").unwrap();
/// println!("File size: {} bytes", size);
/// ```
pub fn get_staged_file_size_from_index(path: &str) -> Result<usize> {
    let content = get_staged_file_content_from_index(path)?;
    Ok(content.len())
}

/// Execute a Git command and capture its output.
///
/// This is a low-level function that runs `git` with the given arguments
/// and returns the stdout as a string. The command is expected to succeed;
/// any non-zero exit code will result in an error.
///
/// # Arguments
///
/// * `args` - Command-line arguments to pass to `git`
///
/// # Returns
///
/// The stdout output of the command, or an error if the command failed.
///
/// # Example
///
/// ```rust,no_run
/// use githook_git::git_capture;
///
/// let branch = git_capture(&["branch", "--show-current"]).unwrap();
/// println!("Current branch: {}", branch.trim());
/// ```
pub fn git_capture(args: &[&str]) -> Result<String> {
    let mut cmd = Command::new("git");
    cmd.args(args);
    let output = cmd.output()?;

    if !output.status.success() {
        bail!(
            "Git command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn git_capture_streaming(args: &[&str]) -> Result<String> {
    use std::io::{BufRead, BufReader};
    use std::process::Stdio;

    let mut cmd = Command::new("git");
    cmd.args(args);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd.spawn()?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("Failed to capture stdout"))?;

    let reader = BufReader::new(stdout);
    let mut result = String::new();

    for line in reader.lines() {
        let line = line?;
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(&line);
    }

    let status = child.wait()?;

    if !status.success() {
        let stderr = if let Some(mut err) = child.stderr {
            use std::io::Read;
            let mut s = String::new();
            err.read_to_string(&mut s).ok();
            s
        } else {
            String::from("Unknown error")
        };
        bail!("Git command failed: {}", stderr);
    }

    Ok(result)
}

pub fn get_author_email() -> Result<String> {
    let output = git_capture(&["config", "user.email"])?;
    Ok(output.trim().to_string())
}

pub fn get_author_name() -> Result<String> {
    let output = git_capture(&["config", "user.name"])?;
    Ok(output.trim().to_string())
}

fn get_glob_regex(pattern: &str) -> Result<Regex> {
    let cache = GLOB_REGEX_CACHE.get_or_init(|| std::sync::Mutex::new(HashMap::new()));

    let mut cache = cache
        .lock()
        .expect("Glob regex cache mutex should not be poisoned");

    if let Some(regex) = cache.get(pattern) {
        return Ok(regex.clone());
    }

    let pattern_regex = glob_to_regex(pattern)?;
    let regex = Regex::new(&pattern_regex)?;
    cache.insert(pattern.to_string(), regex.clone());
    Ok(regex)
}

/// Get a list of staged files matching a pattern.
///
/// This function returns all files that are staged (in the Git index) and
/// match the given glob pattern. The pattern supports wildcards and can
/// include multiple patterns separated by `|`.
///
/// # Arguments
///
/// * `pattern` - A glob pattern to match files (e.g., "*.rs", "src/**/*.js", or "*" for all files)
///
/// # Returns
///
/// A vector of file paths relative to the repository root.
///
/// # Example
///
/// ```rust,no_run
/// use githook_git::get_staged_files;
///
/// // Get all staged Rust files
/// let rust_files = get_staged_files("*.rs").unwrap();
///
/// // Get all staged files
/// let all_files = get_staged_files("*").unwrap();
///
/// // Multiple patterns
/// let files = get_staged_files("*.rs|*.toml").unwrap();
/// ```
pub fn get_staged_files(pattern: &str) -> Result<Vec<String>> {
    let output = git_capture(&["diff", "--cached", "--name-only", "--diff-filter=ACM"])?;

    let files: Vec<String> = output
        .lines()
        .filter(|f| !f.is_empty())
        .map(|s| s.to_string())
        .collect();

    if pattern == "*" {
        return Ok(files);
    }

    let patterns: Vec<&str> = pattern.split('|').collect();

    if patterns.len() == 1 {
        let regex = get_glob_regex(pattern)?;
        return Ok(files.into_iter().filter(|f| regex.is_match(f)).collect());
    }

    let regexes: Result<Vec<Regex>> = patterns.iter().map(|p| get_glob_regex(p.trim())).collect();

    let regexes = regexes?;

    Ok(files
        .into_iter()
        .filter(|f| regexes.iter().any(|r| r.is_match(f)))
        .collect())
}

pub fn is_file_staged(pattern: &str) -> Result<bool> {
    let files = get_staged_files(pattern)?;
    Ok(!files.is_empty())
}

pub fn get_added_files(pattern: &str) -> Result<Vec<String>> {
    let output = git_capture(&["diff", "--cached", "--name-only", "--diff-filter=A"])?;

    let files: Vec<String> = output
        .lines()
        .filter(|f| !f.is_empty())
        .map(|s| s.to_string())
        .collect();

    if pattern == "*" {
        return Ok(files);
    }

    let patterns: Vec<&str> = pattern.split('|').collect();

    if patterns.len() == 1 {
        let regex = get_glob_regex(pattern)?;
        return Ok(files.into_iter().filter(|f| regex.is_match(f)).collect());
    }

    let regexes: Result<Vec<Regex>> = patterns.iter().map(|p| get_glob_regex(p.trim())).collect();

    let regexes = regexes?;

    Ok(files
        .into_iter()
        .filter(|f| regexes.iter().any(|r| r.is_match(f)))
        .collect())
}

pub fn get_deleted_files(pattern: &str) -> Result<Vec<String>> {
    let output = git_capture(&["diff", "--cached", "--name-only", "--diff-filter=D"])?;

    let files: Vec<String> = output
        .lines()
        .filter(|f| !f.is_empty())
        .map(|s| s.to_string())
        .collect();

    if pattern == "*" {
        return Ok(files);
    }

    let patterns: Vec<&str> = pattern.split('|').collect();

    if patterns.len() == 1 {
        let regex = get_glob_regex(pattern)?;
        return Ok(files.into_iter().filter(|f| regex.is_match(f)).collect());
    }

    let regexes: Result<Vec<Regex>> = patterns.iter().map(|p| get_glob_regex(p.trim())).collect();

    let regexes = regexes?;

    Ok(files
        .into_iter()
        .filter(|f| regexes.iter().any(|r| r.is_match(f)))
        .collect())
}

pub fn get_unstaged_files(pattern: &str) -> Result<Vec<String>> {
    let output = git_capture(&["diff", "--name-only", "--diff-filter=ACM"])?;

    let files: Vec<String> = output
        .lines()
        .filter(|f| !f.is_empty())
        .map(|s| s.to_string())
        .collect();

    if pattern == "*" {
        return Ok(files);
    }

    let patterns: Vec<&str> = pattern.split('|').collect();

    if patterns.len() == 1 {
        let regex = get_glob_regex(pattern)?;
        return Ok(files.into_iter().filter(|f| regex.is_match(f)).collect());
    }

    let regexes: Result<Vec<Regex>> = patterns.iter().map(|p| get_glob_regex(p.trim())).collect();

    let regexes = regexes?;

    Ok(files
        .into_iter()
        .filter(|f| regexes.iter().any(|r| r.is_match(f)))
        .collect())
}

pub fn get_modified_files(pattern: &str) -> Result<Vec<String>> {
    let output = git_capture(&["diff", "--name-only", "--diff-filter=M", "HEAD"])?;

    let files: Vec<String> = output
        .lines()
        .filter(|f| !f.is_empty())
        .map(|s| s.to_string())
        .collect();

    if pattern == "*" {
        return Ok(files);
    }

    let patterns: Vec<&str> = pattern.split('|').collect();

    if patterns.len() == 1 {
        let regex = get_glob_regex(pattern)?;
        return Ok(files.into_iter().filter(|f| regex.is_match(f)).collect());
    }

    let regexes: Result<Vec<Regex>> = patterns.iter().map(|p| get_glob_regex(p.trim())).collect();

    let regexes = regexes?;

    Ok(files
        .into_iter()
        .filter(|f| regexes.iter().any(|r| r.is_match(f)))
        .collect())
}

pub fn get_changed_files(pattern: &str) -> Result<Vec<String>> {
    let output = git_capture(&["diff", "--cached", "--name-only", "--diff-filter=M"])?;

    let files: Vec<String> = output
        .lines()
        .filter(|f| !f.is_empty())
        .map(|s| s.to_string())
        .collect();

    if pattern == "*" {
        return Ok(files);
    }

    let patterns: Vec<&str> = pattern.split('|').collect();

    if patterns.len() == 1 {
        let regex = get_glob_regex(pattern)?;
        return Ok(files.into_iter().filter(|f| regex.is_match(f)).collect());
    }

    let regexes: Result<Vec<Regex>> = patterns.iter().map(|p| get_glob_regex(p.trim())).collect();

    let regexes = regexes?;

    Ok(files
        .into_iter()
        .filter(|f| regexes.iter().any(|r| r.is_match(f)))
        .collect())
}

pub fn get_staged_file_content_from_index(file: &str) -> Result<String> {
    git_capture(&["show", &format!(":{}", file)])
}

pub fn get_staged_file_contents_batch(files: &[String]) -> Result<HashMap<String, String>> {
    let mut result = HashMap::with_capacity(files.len());

    if files.is_empty() {
        return Ok(result);
    }

    for file in files {
        if let Ok(content) = get_staged_file_content_from_index(file) {
            result.insert(file.clone(), content);
        }
    }

    Ok(result)
}

pub fn get_staged_file_content(pattern: &str) -> Result<String> {
    let files = get_staged_files(pattern)?;

    if files.is_empty() {
        return Ok(String::new());
    }

    let contents = get_staged_file_contents_batch(&files)?;

    let estimated_size = files.len() * 1024;
    let mut content = String::with_capacity(estimated_size);

    for file in files {
        if let Some(file_content) = contents.get(&file) {
            content.push_str(file_content);
            content.push('\n');
        }
    }

    Ok(content)
}

pub fn get_all_files(pattern: &str) -> Result<Vec<String>> {
    let output = git_capture(&["ls-files"])?;
    let files: Vec<String> = output
        .lines()
        .filter(|f| !f.is_empty())
        .map(|s| s.to_string())
        .collect();

    if pattern == "*" {
        return Ok(files);
    }

    let patterns: Vec<&str> = pattern.split('|').collect();

    if patterns.len() == 1 {
        let regex = get_glob_regex(pattern)?;
        return Ok(files.into_iter().filter(|f| regex.is_match(f)).collect());
    }

    let regexes: Result<Vec<Regex>> = patterns.iter().map(|p| get_glob_regex(p.trim())).collect();

    let regexes = regexes?;

    Ok(files
        .into_iter()
        .filter(|f| regexes.iter().any(|r| r.is_match(f)))
        .collect())
}

pub fn get_diff_added_lines() -> Result<String> {
    let head = git_capture(&["rev-parse", "HEAD"])?;
    let cache_key = format!("diff_added:{}", head.trim());

    {
        let mut cache = get_diff_cache()
            .lock()
            .expect("Diff cache lock should not be poisoned");
        if let Some(cached) = cache.get(&cache_key) {
            return Ok(cached.clone());
        }
    }

    let output = git_capture(&["diff", "--cached"])?;

    let added_lines: Vec<&str> = output
        .lines()
        .filter(|line| line.starts_with('+') && !line.starts_with("+++"))
        .collect();

    let result = added_lines.join("\n");

    {
        let mut cache = get_diff_cache()
            .lock()
            .expect("Diff cache lock should not be poisoned");
        cache.put(cache_key, result.clone());
    }

    Ok(result)
}

pub fn get_added_lines_array() -> Result<Vec<String>> {
    let output = git_capture(&["diff", "--cached"])?;

    let added_lines: Vec<String> = output
        .lines()
        .filter(|line| line.starts_with('+') && !line.starts_with("+++"))
        .map(|line| line[1..].to_string())
        .collect();

    Ok(added_lines)
}

pub fn get_removed_lines_array() -> Result<Vec<String>> {
    let output = git_capture(&["diff", "--cached"])?;

    let removed_lines: Vec<String> = output
        .lines()
        .filter(|line| line.starts_with('-') && !line.starts_with("---"))
        .map(|line| line[1..].to_string())
        .collect();

    Ok(removed_lines)
}

pub fn get_file_diff(path: &str) -> Result<String> {
    git_capture(&["diff", "--cached", "--", path])
}

/// Get statistics about the current staged changes.
///
/// Returns information about files changed, lines added, and lines deleted
/// in the currently staged changes.
///
/// # Returns
///
/// A `DiffStats` struct containing:
/// - `files_changed`: Number of files modified
/// - `additions`: Total lines added
/// - `deletions`: Total lines deleted
///
/// # Example
///
/// ```rust,no_run
/// use githook_git::get_diff_stats;
///
/// let stats = get_diff_stats().unwrap();
/// println!("Files changed: {}", stats.files_changed);
/// println!("Lines added: {}", stats.additions);
/// println!("Lines deleted: {}", stats.deletions);
/// ```
pub fn get_diff_stats() -> Result<DiffStats> {
    let head = git_capture(&["rev-parse", "HEAD"])?;
    let cache_key = format!("diff_stats:{}", head.trim());

    {
        let mut cache = get_diff_cache()
            .lock()
            .expect("Diff cache lock should not be poisoned");
        if let Some(cached) = cache.get(&cache_key) {
            let parts: Vec<&str> = cached.split(',').collect();
            if parts.len() == 3 {
                return Ok(DiffStats {
                    files_changed: parts[0].parse().unwrap_or(0),
                    additions: parts[1].parse().unwrap_or(0),
                    deletions: parts[2].parse().unwrap_or(0),
                });
            }
        }
    }

    let output = git_capture(&["diff", "--cached", "--shortstat"])?;

    let mut stats = DiffStats::default();

    for part in output.split(',') {
        let part = part.trim();
        if part.contains("file") {
            if let Some(num) = part.split_whitespace().next() {
                stats.files_changed = num.parse().unwrap_or(0);
            }
        } else if part.contains("insertion") {
            if let Some(num) = part.split_whitespace().next() {
                stats.additions = num.parse().unwrap_or(0);
            }
        } else if part.contains("deletion")
            && let Some(num) = part.split_whitespace().next()
        {
            stats.deletions = num.parse().unwrap_or(0);
        }
    }

    {
        let mut cache = get_diff_cache()
            .lock()
            .expect("Diff cache lock should not be poisoned");
        let cache_value = format!(
            "{},{},{}",
            stats.files_changed, stats.additions, stats.deletions
        );
        cache.put(cache_key, cache_value);
    }

    Ok(stats)
}

pub fn get_modified_lines() -> Result<usize> {
    let stats = get_diff_stats()?;
    Ok(stats.additions + stats.deletions)
}

/// Get the current Git branch name.
///
/// # Returns
///
/// The name of the current branch (e.g., "main", "develop", "feature/new-feature").
///
/// # Example
///
/// ```rust,no_run
/// use githook_git::get_branch_name;
///
/// let branch = get_branch_name().unwrap();
/// println!("Current branch: {}", branch);
/// ```
pub fn get_branch_name() -> Result<String> {
    git_capture(&["rev-parse", "--abbrev-ref", "HEAD"])
}

pub fn get_current_commit_hash() -> Result<String> {
    git_capture(&["rev-parse", "HEAD"])
}

pub fn get_repo_root() -> Result<String> {
    git_capture(&["rev-parse", "--show-toplevel"])
}

pub fn get_remote_url() -> Result<String> {
    git_capture(&["config", "--get", "remote.origin.url"])
}

pub fn get_staged_blob_oid(path: &str) -> Result<String> {
    let output = git_capture(&["ls-files", "-s", path])?;
    let parts: Vec<&str> = output.split_whitespace().collect();
    let oid = parts.get(1).map(|s| s.to_string()).unwrap_or_default();
    Ok(oid)
}

pub fn is_merge_commit() -> Result<bool> {
    let output = git_capture(&["rev-parse", "--verify", "--quiet", "MERGE_HEAD"]);
    Ok(output.is_ok())
}

pub fn get_merge_head() -> Result<String> {
    git_capture(&["rev-parse", "MERGE_HEAD"])
}

pub fn get_orig_head() -> Result<String> {
    git_capture(&["rev-parse", "ORIG_HEAD"])
}

pub fn get_merge_source_branch() -> Result<String> {
    if let Ok(merge_head) = get_merge_head() {
        if let Ok(branches) = git_capture(&["branch", "-r", "--contains", &merge_head])
            && let Some(first_branch) = branches.lines().next()
        {
            return Ok(first_branch
                .trim()
                .trim_start_matches("origin/")
                .to_string());
        }
        Ok(merge_head)
    } else {
        Ok("unknown".to_string())
    }
}

pub fn has_merge_conflicts() -> Result<bool> {
    let output = git_capture(&["diff", "--name-only", "--diff-filter=U"])?;
    Ok(!output.trim().is_empty())
}

pub fn is_branch_behind(remote_branch: &str) -> Result<bool> {
    match git_capture(&["fetch", "origin", remote_branch]) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Warning: Failed to fetch from remote: {}", e);
            eprintln!("Continuing with local information...");
        }
    }

    let output = git_capture(&["rev-list", "--count", &format!("HEAD..{}", remote_branch)])?;

    let count: usize = output.trim().parse().unwrap_or(0);

    Ok(count > 0)
}

pub fn get_commits_ahead(remote_branch: &str) -> Result<usize> {
    let output = git_capture(&["rev-list", "--count", &format!("{}..HEAD", remote_branch)])?;

    let count = output.trim().parse().unwrap_or(0);

    Ok(count)
}

pub fn get_unpushed_commits() -> Result<Vec<String>> {
    let output = match git_capture(&["log", "@{u}..", "--oneline"]) {
        Ok(out) => out,
        Err(_) => {
            return Ok(Vec::new());
        }
    };

    let commits: Vec<String> = output
        .lines()
        .filter(|line| !line.is_empty())
        .map(|s| s.to_string())
        .collect();

    Ok(commits)
}

pub fn get_commit_message_from_hook_args(hook_args: &[String]) -> Result<String> {
    if hook_args.is_empty() {
        bail!("No commit message file provided in hook args");
    }

    let msg_file = &hook_args[0];

    let cache_key = if let Ok(metadata) = std::fs::metadata(msg_file) {
        let mtime = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        format!("{}:{}", msg_file, mtime)
    } else {
        msg_file.to_string()
    };

    {
        let mut cache = get_commit_msg_cache()
            .lock()
            .expect("Commit message cache lock should not be poisoned");
        if let Some(cached) = cache.get(&cache_key) {
            return Ok(cached.clone());
        }
    }

    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let file = File::open(msg_file)
        .with_context(|| format!("Failed to open commit message file {}", msg_file))?;
    let reader = BufReader::new(file);

    let mut content = String::new();
    for line in reader.lines() {
        let line = line.with_context(|| format!("Failed to read line from {}", msg_file))?;
        if !content.is_empty() {
            content.push('\n');
        }
        content.push_str(&line);
    }

    {
        let mut cache = get_commit_msg_cache()
            .lock()
            .expect("Commit message cache lock should not be poisoned");
        cache.put(cache_key, content.clone());
    }

    Ok(content)
}

pub fn is_author_set() -> Result<bool> {
    let output = git_capture(&["config", "user.name"])?;
    Ok(!output.trim().is_empty())
}

pub fn is_author_email_set() -> Result<bool> {
    let output = git_capture(&["config", "user.email"])?;
    Ok(!output.trim().is_empty())
}

pub fn get_max_file_size() -> Result<f64> {
    let files = get_staged_files("*")?;
    let mut max_size = 0.0;

    for file in files {
        if let Ok(metadata) = std::fs::metadata(&file) {
            let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
            if size_mb > max_size {
                max_size = size_mb;
            }
        }
    }

    Ok(max_size)
}

fn get_secret_patterns() -> &'static [Regex; 6] {
    static PATTERNS: OnceLock<[Regex; 6]> = OnceLock::new();

    PATTERNS.get_or_init(|| {
        [
            Regex::new(r#"(?i)(api[_-]?key|apikey)\s*[:=]\s*['"]?[a-zA-Z0-9]{20,}['"]?"#)
                .expect("Valid regex pattern for API keys"),
            Regex::new(r"AKIA[0-9A-Z]{16}").expect("Valid regex pattern for AWS access keys"),
            Regex::new(r"-----BEGIN (RSA |EC )?PRIVATE KEY-----")
                .expect("Valid regex pattern for private keys"),
            Regex::new(r#"(?i)(password|passwd|pwd)\s*[:=]\s*['"][^'"]{8,}['"]"#)
                .expect("Valid regex pattern for passwords"),
            Regex::new(r#"(?i)(token|secret)\s*[:=]\s*['"]?[a-zA-Z0-9]{20,}['"]?"#)
                .expect("Valid regex pattern for tokens"),
            Regex::new(r"(?i)(postgres|mysql|mongodb)://[^:]+:[^@]+@")
                .expect("Valid regex pattern for database URLs"),
        ]
    })
}

pub fn secrets_with_locations() -> Result<Vec<SecretFinding>> {
    use rayon::prelude::*;

    let files = get_staged_files("*")?;
    let patterns = get_secret_patterns();

    let contents = get_staged_file_contents_batch(&files)?;

    let findings: Vec<SecretFinding> = files
        .par_iter()
        .filter_map(|file| {
            contents.get(file).map(|content| {
                let mut file_findings = Vec::new();

                for (line_num, line) in content.lines().enumerate() {
                    for pattern in patterns.iter() {
                        if pattern.is_match(line) {
                            file_findings.push(SecretFinding {
                                file: file.clone(),
                                line: line_num + 1,
                                line_content: line.to_string(),
                            });
                            break;
                        }
                    }
                }

                file_findings
            })
        })
        .flatten()
        .collect();

    Ok(findings)
}

pub fn contains_secrets() -> Result<bool> {
    Ok(!secrets_with_locations()?.is_empty())
}

fn glob_to_regex(pattern: &str) -> Result<String> {
    let mut regex = String::from("^");
    let mut chars = pattern.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '*' => {
                if chars.peek() == Some(&'*') {
                    chars.next();
                    regex.push_str(".*");
                } else {
                    regex.push_str("[^/]*");
                }
            }
            '?' => regex.push('.'),
            '.' => regex.push_str("\\."),
            '/' => regex.push('/'),
            _ => {
                if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                    regex.push(ch);
                } else {
                    regex.push('\\');
                    regex.push(ch);
                }
            }
        }
    }

    regex.push('$');
    Ok(regex)
}

pub fn get_commit_message() -> Result<String> {
    if let Ok(msg) = std::fs::read_to_string(".git/COMMIT_EDITMSG") {
        let msg = msg
            .lines()
            .filter(|line| !line.starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string();
        return Ok(msg);
    }

    let output = git_capture(&["log", "-1", "--pretty=%B"])?;

    Ok(output.trim().to_string())
}
