//! # githook-git
//!
//! Git helper functions for the Githook scripting language.
//!
//! Provides staged/modified/deleted file queries, diff statistics, branch
//! information, commit message access, and secret-detection scanning.
//! Results are cached with LRU caches and `OnceLock` for efficiency.

use anyhow::{Context, Result, bail};
use lru::LruCache;
use regex::Regex;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::process::Command;
use std::sync::{Mutex, OnceLock};

/// Statistics from the current staged diff.
#[derive(Debug, Default, Clone)]
pub struct DiffStats {
    /// Number of files with changes.
    pub files_changed: usize,
    /// Total lines added.
    pub additions: usize,
    /// Total lines deleted.
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

/// A potential secret detected in staged changes.
#[derive(Debug)]
pub struct SecretFinding {
    /// The file path where the secret was found.
    pub file: String,
    /// The 1-based line number.
    pub line: usize,
    /// The full line content.
    pub line_content: String,
}

/// Returns the size in bytes of a file from the Git staging area (index).
pub fn get_staged_file_size_from_index(path: &str) -> Result<usize> {
    let content = get_staged_file_content_from_index(path)?;
    Ok(content.len())
}

/// Runs a Git command and returns its stdout as a trimmed string.
///
/// Errors if the command exits with a non-zero status.
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

/// Like [`git_capture`] but streams stdout line-by-line for large outputs.
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

/// Returns the configured Git author email.
pub fn get_author_email() -> Result<String> {
    let output = git_capture(&["config", "user.email"])?;
    Ok(output.trim().to_string())
}

/// Returns the configured Git author name.
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

/// Filter a list of file paths by a glob pattern (or multi-pattern separated
/// by `|`). Returns all files when `pattern` is `"*"`.
fn filter_files_by_pattern(files: Vec<String>, pattern: &str) -> Result<Vec<String>> {
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

/// Collect non-empty lines from git output into a `Vec<String>`.
fn collect_file_lines(output: &str) -> Vec<String> {
    output
        .lines()
        .filter(|f| !f.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// List files from a `git diff` command filtered by the given diff-filter and
/// then matched against a glob pattern.
fn git_diff_files(diff_args: &[&str], pattern: &str) -> Result<Vec<String>> {
    let output = git_capture(diff_args)?;
    let files = collect_file_lines(&output);
    filter_files_by_pattern(files, pattern)
}

/// Returns staged (added/copied/modified) file paths matching the glob pattern.
pub fn get_staged_files(pattern: &str) -> Result<Vec<String>> {
    git_diff_files(
        &["diff", "--cached", "--name-only", "--diff-filter=ACM"],
        pattern,
    )
}

/// Returns `true` if any staged file matches the given pattern.
pub fn is_file_staged(pattern: &str) -> Result<bool> {
    let files = get_staged_files(pattern)?;
    Ok(!files.is_empty())
}

/// Returns newly added file paths matching the glob pattern.
pub fn get_added_files(pattern: &str) -> Result<Vec<String>> {
    git_diff_files(
        &["diff", "--cached", "--name-only", "--diff-filter=A"],
        pattern,
    )
}

/// Returns deleted file paths matching the glob pattern.
pub fn get_deleted_files(pattern: &str) -> Result<Vec<String>> {
    git_diff_files(
        &["diff", "--cached", "--name-only", "--diff-filter=D"],
        pattern,
    )
}

/// Returns unstaged (working-tree) file paths matching the glob pattern.
pub fn get_unstaged_files(pattern: &str) -> Result<Vec<String>> {
    git_diff_files(&["diff", "--name-only", "--diff-filter=ACM"], pattern)
}

/// Returns modified file paths matching the glob pattern.
pub fn get_modified_files(pattern: &str) -> Result<Vec<String>> {
    git_diff_files(&["diff", "--name-only", "--diff-filter=M", "HEAD"], pattern)
}

/// Returns staged-modified file paths matching the glob pattern.
pub fn get_changed_files(pattern: &str) -> Result<Vec<String>> {
    git_diff_files(
        &["diff", "--cached", "--name-only", "--diff-filter=M"],
        pattern,
    )
}

/// Returns the content of a file from the Git index (staging area).
pub fn get_staged_file_content_from_index(file: &str) -> Result<String> {
    git_capture(&["show", &format!(":{}", file)])
}

/// Reads the staged content of multiple files in one pass.
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

/// Returns concatenated staged content of all files matching the pattern.
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

/// Returns all tracked file paths matching the glob pattern.
pub fn get_all_files(pattern: &str) -> Result<Vec<String>> {
    let output = git_capture(&["ls-files"])?;
    let files = collect_file_lines(&output);
    filter_files_by_pattern(files, pattern)
}

/// Returns the raw added lines (`+` lines) from the staged diff.
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

/// Returns added lines from the staged diff as an array of strings.
pub fn get_added_lines_array() -> Result<Vec<String>> {
    let output = git_capture(&["diff", "--cached"])?;

    let added_lines: Vec<String> = output
        .lines()
        .filter(|line| line.starts_with('+') && !line.starts_with("+++"))
        .map(|line| line[1..].to_string())
        .collect();

    Ok(added_lines)
}

/// Returns removed lines from the staged diff as an array of strings.
pub fn get_removed_lines_array() -> Result<Vec<String>> {
    let output = git_capture(&["diff", "--cached"])?;

    let removed_lines: Vec<String> = output
        .lines()
        .filter(|line| line.starts_with('-') && !line.starts_with("---"))
        .map(|line| line[1..].to_string())
        .collect();

    Ok(removed_lines)
}

/// Returns the staged diff for a specific file.
pub fn get_file_diff(path: &str) -> Result<String> {
    git_capture(&["diff", "--cached", "--", path])
}

/// Returns aggregate diff statistics (files changed, additions, deletions).
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

/// Returns total modified lines (additions + deletions).
pub fn get_modified_lines() -> Result<usize> {
    let stats = get_diff_stats()?;
    Ok(stats.additions + stats.deletions)
}

/// Returns the current branch name.
pub fn get_branch_name() -> Result<String> {
    git_capture(&["rev-parse", "--abbrev-ref", "HEAD"])
}

/// Returns the SHA hash of the current HEAD commit.
pub fn get_current_commit_hash() -> Result<String> {
    git_capture(&["rev-parse", "HEAD"])
}

/// Returns the absolute path to the repository root.
pub fn get_repo_root() -> Result<String> {
    git_capture(&["rev-parse", "--show-toplevel"])
}

/// Returns the `origin` remote URL.
pub fn get_remote_url() -> Result<String> {
    git_capture(&["config", "--get", "remote.origin.url"])
}

/// Returns the Git blob OID for a staged file.
pub fn get_staged_blob_oid(path: &str) -> Result<String> {
    let output = git_capture(&["ls-files", "-s", path])?;
    let parts: Vec<&str> = output.split_whitespace().collect();
    let oid = parts.get(1).map(|s| s.to_string()).unwrap_or_default();
    Ok(oid)
}

/// Returns `true` if the current commit is a merge commit.
pub fn is_merge_commit() -> Result<bool> {
    let output = git_capture(&["rev-parse", "--verify", "--quiet", "MERGE_HEAD"]);
    Ok(output.is_ok())
}

/// Returns the MERGE_HEAD commit hash.
pub fn get_merge_head() -> Result<String> {
    git_capture(&["rev-parse", "MERGE_HEAD"])
}

/// Returns the ORIG_HEAD commit hash.
pub fn get_orig_head() -> Result<String> {
    git_capture(&["rev-parse", "ORIG_HEAD"])
}

/// Returns the source branch of an in-progress merge.
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

/// Returns `true` if there are unresolved merge conflicts.
pub fn has_merge_conflicts() -> Result<bool> {
    let output = git_capture(&["diff", "--name-only", "--diff-filter=U"])?;
    Ok(!output.trim().is_empty())
}

/// Returns `true` if HEAD is behind the given remote branch.
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

/// Returns the number of commits HEAD is ahead of the given remote branch.
pub fn get_commits_ahead(remote_branch: &str) -> Result<usize> {
    let output = git_capture(&["rev-list", "--count", &format!("{}..HEAD", remote_branch)])?;

    let count = output.trim().parse().unwrap_or(0);

    Ok(count)
}

/// Returns one-line summaries of commits not yet pushed to the upstream.
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

/// Reads the commit message from the file path passed as a hook argument.
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

/// Returns `true` if `user.name` is configured.
pub fn is_author_set() -> Result<bool> {
    let output = git_capture(&["config", "user.name"])?;
    Ok(!output.trim().is_empty())
}

/// Returns `true` if `user.email` is configured.
pub fn is_author_email_set() -> Result<bool> {
    let output = git_capture(&["config", "user.email"])?;
    Ok(!output.trim().is_empty())
}

/// Returns the size (in MB) of the largest staged file.
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

/// Scans staged file contents for potential secrets (API keys, tokens, etc.).
///
/// Returns detailed findings with file, line number, and content.
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

/// Returns `true` if any staged file contains potential secrets.
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

/// Returns the current commit message from `.git/COMMIT_EDITMSG` or the last commit.
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_glob_to_regex_star_rs() {
        let re = glob_to_regex("*.rs").unwrap();
        let regex = Regex::new(&re).unwrap();
        assert!(regex.is_match("main.rs"));
        assert!(regex.is_match("lib.rs"));
        assert!(!regex.is_match("src/main.rs"), "single * must not cross /");
        assert!(!regex.is_match("main.toml"));
    }

    #[test]
    fn test_glob_to_regex_double_star() {
        let re = glob_to_regex("**/*.rs").unwrap();
        let regex = Regex::new(&re).unwrap();
        assert!(regex.is_match("src/main.rs"));
        assert!(regex.is_match("crates/githook/src/lib.rs"));
        assert!(!regex.is_match("README.md"));
    }

    #[test]
    fn test_glob_to_regex_question_mark() {
        let re = glob_to_regex("?.rs").unwrap();
        let regex = Regex::new(&re).unwrap();
        assert!(regex.is_match("a.rs"));
        assert!(!regex.is_match("ab.rs"));
    }

    #[test]
    fn test_glob_to_regex_literal() {
        let re = glob_to_regex("Cargo.toml").unwrap();
        let regex = Regex::new(&re).unwrap();
        assert!(regex.is_match("Cargo.toml"));
        assert!(!regex.is_match("Cargo.lock"));
        assert!(!regex.is_match("src/Cargo.toml"));
    }

    #[test]
    fn test_glob_to_regex_nested_path() {
        let re = glob_to_regex("src/*.rs").unwrap();
        let regex = Regex::new(&re).unwrap();
        assert!(regex.is_match("src/main.rs"));
        assert!(regex.is_match("src/lib.rs"));
        assert!(
            !regex.is_match("src/sub/mod.rs"),
            "single * must not cross /"
        );
        assert!(!regex.is_match("main.rs"));
    }

    #[test]
    fn test_filter_wildcard_returns_all() {
        let files = vec!["a.rs".into(), "b.py".into(), "c.js".into()];
        let result = filter_files_by_pattern(files.clone(), "*").unwrap();
        assert_eq!(result, files);
    }

    #[test]
    fn test_filter_single_pattern() {
        let files = vec!["main.rs".into(), "lib.rs".into(), "package.json".into()];
        let result = filter_files_by_pattern(files, "*.rs").unwrap();
        assert_eq!(result, vec!["main.rs", "lib.rs"]);
    }

    #[test]
    fn test_filter_multi_pattern() {
        let files = vec![
            "main.rs".into(),
            "style.css".into(),
            "index.js".into(),
            "README.md".into(),
        ];
        let result = filter_files_by_pattern(files, "*.rs | *.js").unwrap();
        assert_eq!(result, vec!["main.rs", "index.js"]);
    }

    #[test]
    fn test_filter_no_matches() {
        let files = vec!["main.rs".into(), "lib.rs".into()];
        let result = filter_files_by_pattern(files, "*.py").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_collect_file_lines_filters_empty() {
        let output = "foo.rs\n\nbar.rs\n\n";
        let result = collect_file_lines(output);
        assert_eq!(result, vec!["foo.rs", "bar.rs"]);
    }

    #[test]
    fn test_collect_file_lines_empty_input() {
        let result = collect_file_lines("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_secret_pattern_api_key() {
        let patterns = get_secret_patterns();
        assert!(patterns[0].is_match(r#"API_KEY = "abcdefghij1234567890""#));
        assert!(patterns[0].is_match(r#"api-key: 'ABCDEFGHIJKLMNOPQRSTUVWX'"#));
        assert!(!patterns[0].is_match(r#"let name = "hello""#));
    }

    #[test]
    fn test_secret_pattern_aws_key() {
        let patterns = get_secret_patterns();
        assert!(patterns[1].is_match("AKIAIOSFODNN7EXAMPLE"));
        assert!(!patterns[1].is_match("NOTAVALIDAWSKEY"));
    }

    #[test]
    fn test_secret_pattern_private_key() {
        let patterns = get_secret_patterns();
        assert!(patterns[2].is_match("-----BEGIN RSA PRIVATE KEY-----"));
        assert!(patterns[2].is_match("-----BEGIN PRIVATE KEY-----"));
        assert!(patterns[2].is_match("-----BEGIN EC PRIVATE KEY-----"));
        assert!(!patterns[2].is_match("-----BEGIN PUBLIC KEY-----"));
    }

    #[test]
    fn test_secret_pattern_password() {
        let patterns = get_secret_patterns();
        assert!(patterns[3].is_match(r#"password = "mysecretpass123""#));
        assert!(patterns[3].is_match(r#"passwd: 'longpassword'"#));
        assert!(
            !patterns[3].is_match(r#"password = "short""#),
            "under 8 chars"
        );
    }

    #[test]
    fn test_secret_pattern_token() {
        let patterns = get_secret_patterns();
        assert!(patterns[4].is_match(r#"token = "abcdefghij1234567890""#));
        assert!(patterns[4].is_match(r#"SECRET: abcdefghij1234567890abcde"#));
    }

    #[test]
    fn test_secret_pattern_database_url() {
        let patterns = get_secret_patterns();
        assert!(patterns[5].is_match("postgres://admin:password@localhost/db"));
        assert!(patterns[5].is_match("mysql://root:pass@127.0.0.1/mydb"));
        assert!(patterns[5].is_match("mongodb://user:secret@host/collection"));
        assert!(!patterns[5].is_match("https://example.com"));
    }

    #[test]
    fn test_diff_stats_default() {
        let stats = DiffStats::default();
        assert_eq!(stats.files_changed, 0);
        assert_eq!(stats.additions, 0);
        assert_eq!(stats.deletions, 0);
    }

    #[test]
    fn test_glob_double_star_deep_path() {
        let re = glob_to_regex("**/test_*.py").unwrap();
        let regex = Regex::new(&re).unwrap();
        assert!(regex.is_match("tests/test_main.py"));
        assert!(regex.is_match("a/b/c/test_utils.py"));
        assert!(!regex.is_match("tests/main.py"));
    }

    #[test]
    fn test_glob_special_chars_escaped() {
        let re = glob_to_regex("file[1].txt").unwrap();
        let regex = Regex::new(&re).unwrap();
        assert!(regex.is_match("file[1].txt"));
    }
}
