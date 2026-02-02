use anyhow::Result;
use githook_macros::{callable_impl, docs};
use std::{
    fmt,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct FilesCollection {
    pub staged: Vec<String>,
    pub all: Vec<String>,
    pub modified: Vec<String>,
    pub added: Vec<String>,
    pub deleted: Vec<String>,
    pub unstaged: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DiffCollection {
    pub added_lines: Vec<String>,
    pub removed_lines: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MergeContext {
    pub source: String,
    pub target: String,
}

impl FilesCollection {
    pub fn staged(&self) -> Vec<String> {
        self.staged.clone()
    }

    pub fn all(&self) -> Vec<String> {
        self.all.clone()
    }

    pub fn modified(&self) -> Vec<String> {
        self.modified.clone()
    }

    pub fn added(&self) -> Vec<String> {
        self.added.clone()
    }

    pub fn deleted(&self) -> Vec<String> {
        self.deleted.clone()
    }

    pub fn unstaged(&self) -> Vec<String> {
        self.unstaged.clone()
    }
}

impl DiffCollection {
    pub fn added_lines(&self) -> Vec<String> {
        self.added_lines.clone()
    }

    pub fn removed_lines(&self) -> Vec<String> {
        self.removed_lines.clone()
    }
}

impl MergeContext {
    pub fn source(&self) -> String {
        self.source.clone()
    }

    pub fn target(&self) -> String {
        self.target.clone()
    }
}

#[derive(Debug, Clone)]
pub struct PathContext {
    path_buf: PathBuf,
}

impl PathContext {
    pub fn from_path(path: impl AsRef<Path>) -> Self {
        Self {
            path_buf: path.as_ref().to_path_buf(),
        }
    }

    pub fn as_str(&self) -> &str {
        self.path_buf.to_str().unwrap_or("")
    }
}

impl fmt::Display for PathContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path_buf.display())
    }
}

#[callable_impl]
impl PathContext {
    #[docs(
        name = "path.string",
        description = "Full path as string",
        example = "print file.path.string"
    )]
    #[property]
    pub fn string(&self) -> String {
        self.path_buf.to_string_lossy().to_string()
    }

    #[docs(
        name = "path.basename",
        description = "Base name of the file without extension",
        example = "print file.path.basename"
    )]
    #[property]
    pub fn basename(&self) -> String {
        self.path_buf
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string()
    }

    #[docs(
        name = "path.extension",
        description = "File extension",
        example = "print file.path.extension"
    )]
    #[property]
    pub fn extension(&self) -> String {
        self.path_buf
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_string())
            .unwrap_or_default()
    }

    #[docs(
        name = "path.parent",
        description = "Parent directory path",
        example = "print \"Dir: \" + file.path.parent"
    )]
    #[property]
    pub fn parent(&self) -> String {
        self.path_buf
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("")
            .to_string()
    }

    #[docs(
        name = "path.filename",
        description = "File name with extension",
        example = "print file.path.filename"
    )]
    #[property]
    pub fn filename(&self) -> String {
        self.path_buf
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string()
    }

    #[docs(
        name = "join",
        description = "Joins path components",
        example = "file.path.join(\"subdir\")"
    )]
    #[method]
    pub fn join(&self, other: &str) -> String {
        self.path_buf.join(other).to_string_lossy().to_string()
    }
}

#[derive(Debug, Clone)]
pub struct FileContext {
    pub path: PathContext,
    path_buf: PathBuf,
}

impl FileContext {
    pub fn from_path(path: impl AsRef<Path>) -> Self {
        let path_buf = path.as_ref().to_path_buf();
        let path_ctx = PathContext::from_path(&path_buf);
        Self {
            path: path_ctx,
            path_buf,
        }
    }
}

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[callable_impl]
impl FileContext {
    #[docs(
        name = "file.name",
        description = "File name with extension",
        example = "print file.name"
    )]
    #[property]
    pub fn name(&self) -> String {
        self.path_buf
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string()
    }

    #[docs(
        name = "file.basename",
        description = "Base name of the file without extension",
        example = "print file.basename"
    )]
    #[property]
    pub fn basename(&self) -> String {
        self.path_buf
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string()
    }

    #[docs(
        name = "file.extension",
        description = "File extension",
        example = "print file.extension"
    )]
    #[property]
    pub fn extension(&self) -> String {
        self.path_buf
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_string())
            .unwrap_or_default()
    }

    #[docs(
        name = "file.dirname",
        description = "Directory name of the file",
        example = "print file.dirname"
    )]
    #[property]
    pub fn dirname(&self) -> String {
        self.path_buf
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("")
            .to_string()
    }

    #[docs(
        name = "file.diff",
        description = "Staged diff for this file",
        example = "if file.diff.contains(\"TODO\") { warn \"TODO in diff\" }"
    )]
    #[property]
    pub fn diff(&self) -> String {
        githook_git::get_file_diff(&self.path.string()).unwrap_or_default()
    }

    #[docs(
        name = "file.content",
        description = "Content of the file as a string",
        example = "if file.content.contains(\"TODO\") { warn \"TODO found\" }"
    )]
    #[property]
    pub fn content(&self) -> String {
        std::fs::read_to_string(&self.path_buf).unwrap_or_default()
    }

    #[docs(
        name = "file.size",
        description = "Size of the file in bytes",
        example = "print file.size"
    )]
    #[property]
    pub fn size(&self) -> f64 {
        std::fs::metadata(&self.path_buf)
            .map(|m| m.len() as f64)
            .unwrap_or(0.0)
    }

    #[docs(
        name = "exists",
        description = "Checks if the file exists",
        example = "if file.exists { print \"File exists\" }"
    )]
    #[method]
    pub fn exists(&self) -> bool {
        self.path_buf.exists()
    }

    #[docs(
        name = "file.is_file",
        description = "Checks if the path is a file",
        example = "if file.is_file { print \"It's a file\" }"
    )]
    #[method]
    pub fn is_file(&self) -> bool {
        self.path_buf.is_file()
    }

    #[docs(
        name = "file.is_dir",
        description = "Checks if the path is a directory",
        example = "if file.is_dir { print \"It's a directory\" }"
    )]
    #[method]
    pub fn is_dir(&self) -> bool {
        self.path_buf.is_dir()
    }

    #[docs(
        name = "file.is_readable",
        description = "Checks if the file is readable",
        example = "if file.is_readable { print \"File is readable\" }"
    )]
    #[method]
    pub fn is_readable(&self) -> bool {
        self.path_buf.metadata().is_ok()
    }

    #[docs(
        name = "file.is_writable",
        description = "Checks if the file is writable",
        example = "if file.is_writable { print \"File is writable\" }"
    )]
    #[method]
    pub fn is_executable(&self) -> bool {
        #[cfg(unix)]
        {
            if let Ok(metadata) = std::fs::metadata(&self.path_buf) {
                metadata.permissions().mode() & 0o111 != 0
            } else {
                false
            }
        }
        #[cfg(not(unix))]
        false
    }

    #[docs(
        name = "file.is_symlink",
        description = "Checks if the file is a symbolic link",
        example = "if file.is_symlink { print \"It's a symlink\" }"
    )]
    #[method]
    pub fn is_symlink(&self) -> bool {
        self.path_buf.is_symlink()
    }

    #[docs(
        name = "file.is_absolute",
        description = "Checks if the path is absolute",
        example = "if file.is_absolute { print \"Absolute path\" }"
    )]
    #[method]
    pub fn is_absolute(&self) -> bool {
        self.path_buf.is_absolute()
    }

    #[docs(
        name = "file.is_relative",
        description = "Checks if the path is relative",
        example = "if file.is_relative { print \"Relative path\" }"
    )]
    #[method]
    pub fn is_relative(&self) -> bool {
        self.path_buf.is_relative()
    }

    #[docs(
        name = "file.modified_time",
        description = "Last modified time of the file as a UNIX timestamp",
        example = "print file.modified_time"
    )]
    #[method]
    pub fn modified_time(&self) -> u64 {
        self.path_buf
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    #[docs(
        name = "file.created_time",
        description = "Creation time of the file as a UNIX timestamp",
        example = "print file.created_time"
    )]
    #[method]
    pub fn created_time(&self) -> u64 {
        self.path_buf
            .metadata()
            .and_then(|m| m.created())
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    #[docs(
        name = "file.is_hidden",
        description = "Checks if the file is hidden",
        example = "if file.is_hidden { print \"Hidden file\" }"
    )]
    #[method]
    pub fn is_hidden(&self) -> bool {
        self.path_buf
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name_str| name_str.starts_with('.'))
            .unwrap_or(false)
    }

    #[docs(
        name = "file.permissions",
        description = "File permissions (Unix only)",
        example = "print file.permissions"
    )]
    #[method]
    pub fn permissions(&self) -> u32 {
        #[cfg(unix)]
        {
            self.path_buf
                .metadata()
                .map(|m| m.permissions().mode())
                .unwrap_or(0)
        }
        #[cfg(not(unix))]
        0
    }

    #[docs(
        name = "file.readable",
        description = "Checks if the file is readable",
        example = "if file.readable { print \"File is readable\" }"
    )]
    #[method]
    pub fn contains(&self, pattern: &str) -> bool {
        self.path.string().contains(pattern)
    }

    #[docs(
        name = "file.starts_with",
        description = "Checks if the file path starts with a prefix",
        example = "if file.path.starts_with(\"src/\") { print \"In src directory\" }"
    )]
    #[method]
    pub fn starts_with(&self, prefix: &str) -> bool {
        self.path.string().starts_with(prefix)
    }

    #[docs(
        name = "file.ends_with",
        description = "Checks if the file path ends with a suffix",
        example = "if file.path.ends_with(\".rs\") { print \"Rust source file\" }"
    )]
    #[method]
    pub fn ends_with(&self, suffix: &str) -> bool {
        self.path.string().ends_with(suffix)
    }
}

#[derive(Debug, Clone)]
pub struct GitContext {
    pub branch: BranchInfo,
    pub commit: Option<CommitInfo>,
    pub author: AuthorInfo,
    pub remote: RemoteInfo,
    pub stats: DiffStats,
    pub files: FilesCollection,
    pub diff: DiffCollection,
    pub is_merge_commit: bool,
    pub has_conflicts: bool,
}

#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub message: String,
    pub hash: String,
}

#[derive(Debug, Clone)]
pub struct AuthorInfo {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone)]
pub struct RemoteInfo {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Default, Clone)]
pub struct DiffStats {
    pub files_changed: usize,
    pub additions: usize,
    pub deletions: usize,
}

impl Default for GitContext {
    fn default() -> Self {
        Self::new()
    }
}

impl GitContext {
    pub fn new() -> Self {
        let git_stats = githook_git::get_diff_stats().unwrap_or_default();

        Self {
            branch: BranchInfo {
                name: githook_git::get_branch_name().unwrap_or_else(|_| "main".to_string()),
            },
            commit: None,
            author: AuthorInfo {
                name: githook_git::get_author_name().unwrap_or_default(),
                email: githook_git::get_author_email().unwrap_or_default(),
            },
            remote: RemoteInfo {
                name: "origin".to_string(),
                url: githook_git::get_remote_url().unwrap_or_default(),
            },
            stats: DiffStats {
                files_changed: git_stats.files_changed,
                additions: git_stats.additions,
                deletions: git_stats.deletions,
            },
            files: FilesCollection {
                staged: githook_git::get_staged_files("*").unwrap_or_default(),
                all: githook_git::get_all_files("*").unwrap_or_default(),
                modified: githook_git::get_modified_files("*").unwrap_or_default(),
                added: githook_git::get_added_files("*").unwrap_or_default(),
                deleted: githook_git::get_deleted_files("*").unwrap_or_default(),
                unstaged: githook_git::get_unstaged_files("*").unwrap_or_default(),
            },
            diff: DiffCollection {
                added_lines: githook_git::get_added_lines_array().unwrap_or_default(),
                removed_lines: githook_git::get_removed_lines_array().unwrap_or_default(),
            },
            is_merge_commit: githook_git::is_merge_commit().unwrap_or(false),
            has_conflicts: false,
        }
    }
}

#[callable_impl]
impl GitContext {
    #[docs(
        name = "git.branch",
        description = "Current Git branch information",
        example = "print git.branch.name"
    )]
    #[property]
    pub fn is_merge_commit(&self) -> bool {
        self.is_merge_commit
    }

    #[docs(
        name = "git.has_conflicts",
        description = "Checks if there are merge conflicts",
        example = "if git.has_conflicts { warn \"Resolve conflicts first\" }"
    )]
    #[property]
    pub fn has_conflicts(&self) -> bool {
        self.has_conflicts
    }
}

#[callable_impl]
impl BranchInfo {
    #[docs(
        name = "git.branch.name",
        description = "Current Git branch name",
        example = "if git.branch.name == \"main\" { warn \"Direct push to main\" }"
    )]
    #[property]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[docs(
        name = "is_main",
        description = "Checks if the current branch is main or master",
        example = "if git.branch.is_main { print \"On main branch\" }"
    )]
    #[property]
    pub fn is_main(&self) -> bool {
        matches!(self.name.as_str(), "main" | "master")
    }
}

#[callable_impl]
impl CommitInfo {
    #[docs(
        name = "git.commit.message",
        description = "Git commit message",
        example = "print git.commit.message"
    )]
    #[property]
    pub fn message(&self) -> String {
        self.message.clone()
    }

    #[docs(
        name = "git.commit.hash",
        description = "Git commit hash",
        example = "print git.commit.hash"
    )]
    #[property]
    pub fn hash(&self) -> String {
        self.hash.clone()
    }
}

#[callable_impl]
impl AuthorInfo {
    #[docs(
        name = "git.author.name",
        description = "Git author name",
        example = "print git.author.name"
    )]
    #[property]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[docs(
        name = "git.author.email",
        description = "Git author email address",
        example = "print git.author.email"
    )]
    #[property]
    pub fn email(&self) -> String {
        self.email.clone()
    }
}

#[callable_impl]
impl RemoteInfo {
    #[docs(
        name = "git.remote.name",
        description = "Git remote name",
        example = "print git.remote.name"
    )]
    #[property]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[docs(
        name = "git.remote.url",
        description = "Git remote URL",
        example = "print git.remote.url"
    )]
    #[property]
    pub fn url(&self) -> String {
        self.url.clone()
    }
}

#[callable_impl]
impl DiffStats {
    #[docs(
        name = "git.stats.files_changed",
        description = "Number of files changed in the diff",
        example = "print git.stats.files_changed"
    )]
    #[property]
    pub fn files_changed(&self) -> f64 {
        self.files_changed as f64
    }

    #[docs(
        name = "git.stats.additions",
        description = "Number of additions in the diff",
        example = "print git.stats.additions"
    )]
    #[property]
    pub fn additions(&self) -> f64 {
        self.additions as f64
    }

    #[docs(
        name = "git.stats.deletions",
        description = "Number of deletions in the diff",
        example = "print git.stats.deletions"
    )]
    #[property]
    pub fn deletions(&self) -> f64 {
        self.deletions as f64
    }

    #[docs(
        name = "git.stats.modified_lines",
        description = "Total number of modified lines (additions + deletions)",
        example = "print git.stats.modified_lines"
    )]
    #[property]
    pub fn modified_lines(&self) -> f64 {
        (self.additions + self.deletions) as f64
    }
}

#[derive(Debug, Clone)]
pub struct StringContext {
    value: String,
}

impl StringContext {
    pub fn new(value: String) -> Self {
        Self { value }
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

#[callable_impl]
impl StringContext {
    #[docs(
        name = "string.length",
        description = "Length of the string",
        example = "print \"hello\".length"
    )]
    #[property]
    pub fn length(&self) -> f64 {
        self.value.len() as f64
    }

    #[docs(
        name = "string.upper",
        description = "Converts string to uppercase",
        example = "\"hello\".upper // \"HELLO\""
    )]
    #[property]
    pub fn upper(&self) -> String {
        self.value.to_uppercase()
    }

    #[docs(
        name = "string.lower",
        description = "Converts string to lowercase",
        example = "\"HELLO\".lower // \"hello\""
    )]
    #[property]
    pub fn lower(&self) -> String {
        self.value.to_lowercase()
    }

    #[docs(
        name = "string.reverse",
        description = "Reverses the string",
        example = "\"hello\".reverse // \"olleh\""
    )]
    #[method]
    pub fn reverse(&self) -> String {
        self.value.chars().rev().collect()
    }

    #[docs(
        name = "string.len",
        description = "Returns the length of the string",
        example = "\"hello\".len // 5"
    )]
    #[method]
    pub fn len(&self) -> f64 {
        self.value.len() as f64
    }

    #[docs(
        name = "string.is_empty",
        description = "Checks if the string is empty",
        example = "\"\".is_empty // true"
    )]
    #[method]
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    #[docs(
        name = "string.to_lowercase",
        description = "Converts string to lowercase",
        example = "\"HELLO\".to_lowercase() // \"hello\""
    )]
    #[method]
    pub fn to_lowercase(&self) -> String {
        self.value.to_lowercase()
    }

    #[docs(
        name = "string.to_uppercase",
        description = "Converts string to uppercase",
        example = "\"hello\".to_uppercase() // \"HELLO\""
    )]
    #[method]
    pub fn to_uppercase(&self) -> String {
        self.value.to_uppercase()
    }

    #[docs(
        name = "string.trim",
        description = "Removes leading and trailing whitespace",
        example = "\"  hello  \".trim // \"hello\""
    )]
    #[method]
    pub fn trim(&self) -> String {
        self.value.trim().to_string()
    }

    #[docs(
        name = "string.replace",
        description = "Replaces occurrences of a substring with another",
        example = "\"hello world\".replace(\"world\", \"there\") // \"hello there\""
    )]
    #[method]
    pub fn replace(&self, from: &str, to: &str) -> String {
        self.value.replace(from, to)
    }

    #[docs(
        name = "string.contains",
        description = "Checks if the string contains a substring",
        example = "\"hello world\".contains(\"world\") // true"
    )]
    #[method]
    pub fn contains(&self, needle: &str) -> bool {
        self.value.contains(needle)
    }

    #[docs(
        name = "string.starts_with",
        description = "Checks if the string starts with a prefix",
        example = "\"hello world\".starts_with(\"hello\") // true"
    )]
    #[method]
    pub fn starts_with(&self, prefix: &str) -> bool {
        self.value.starts_with(prefix)
    }

    #[docs(
        name = "string.ends_with",
        description = "Checks if the string ends with a suffix",
        example = "\"hello world\".ends_with(\"world\") // true"
    )]
    #[method]
    pub fn ends_with(&self, suffix: &str) -> bool {
        self.value.ends_with(suffix)
    }

    #[docs(
        name = "string.matches",
        description = "Checks if the string matches a regex pattern",
        example = "\"hello123\".matches(\"^hello\\\\d+$\") // true"
    )]
    #[method]
    pub fn matches(&self, pattern: &str) -> bool {
        regex::Regex::new(pattern)
            .map(|re| re.is_match(&self.value))
            .unwrap_or(false)
    }

    #[docs(
        name = "string.split",
        description = "Splits the string by a delimiter",
        example = "\"a,b,c\".split(\",\") // [\"a\", \"b\", \"c\"]"
    )]
    #[method]
    pub fn split(&self, delimiter: &str) -> Vec<String> {
        self.value.split(delimiter).map(|s| s.to_string()).collect()
    }

    #[docs(
        name = "string.lines",
        description = "Splits the string into lines",
        example = "\"line1\\nline2\".lines // [\"line1\", \"line2\"]"
    )]
    #[method]
    pub fn lines(&self) -> Vec<String> {
        self.value.lines().map(|s| s.to_string()).collect()
    }
}

#[derive(Debug, Clone)]
pub struct NumberContext {
    value: f64,
}

impl NumberContext {
    pub fn new(value: f64) -> Self {
        Self { value }
    }

    pub fn value(&self) -> f64 {
        self.value
    }
}

#[callable_impl]
impl NumberContext {
    #[docs(
        name = "number.abs",
        description = "Absolute value of the number",
        example = "print (-5).abs"
    )]
    #[method]
    pub fn abs(&self) -> f64 {
        self.value.abs()
    }

    #[docs(
        name = "number.floor",
        description = "Floor of the number",
        example = "print 3.7.floor"
    )]
    #[method]
    pub fn floor(&self) -> f64 {
        self.value.floor()
    }

    #[docs(
        name = "number.ceil",
        description = "Ceiling of the number",
        example = "print 3.3.ceil"
    )]
    #[method]
    pub fn ceil(&self) -> f64 {
        self.value.ceil()
    }

    #[docs(
        name = "number.round",
        description = "Rounds the number to the nearest integer",
        example = "print 3.5.round"
    )]
    #[method]
    pub fn round(&self) -> f64 {
        self.value.round()
    }

    #[docs(
        name = "number.sqrt",
        description = "Square root of the number",
        example = "print 16.sqrt // 4"
    )]
    #[method]
    pub fn sqrt(&self) -> f64 {
        self.value.sqrt()
    }

    #[docs(
        name = "number.pow",
        description = "Raises the number to the power of exp",
        example = "print 2.pow(3) // 8"
    )]
    #[method]
    pub fn pow(&self, exp: f64) -> f64 {
        self.value.powf(exp)
    }

    #[docs(
        name = "number.sin",
        description = "Sine of the number (in radians)",
        example = "print (3.14159 / 2).sin() // ~1"
    )]
    #[method]
    pub fn sin(&self) -> f64 {
        self.value.sin()
    }

    #[docs(
        name = "number.cos",
        description = "Cosine of the number (in radians)",
        example = "print 0.0.cos() // 1"
    )]
    #[method]
    pub fn cos(&self) -> f64 {
        self.value.cos()
    }

    #[docs(
        name = "number.tan",
        description = "Tangent of the number (in radians)",
        example = "print 0.0.tan() // 0"
    )]
    #[method]
    pub fn tan(&self) -> f64 {
        self.value.tan()
    }

    #[docs(
        name = "number.percent",
        description = "Converts a decimal number to percentage",
        example = "print 0.85.percent() // 85.0"
    )]
    #[method]
    pub fn percent(&self) -> f64 {
        self.value * 100.0
    }
}

#[derive(Debug, Clone)]
pub struct ArrayContext {
    items: Vec<crate::value::Value>,
}

impl ArrayContext {
    pub fn new(items: Vec<crate::value::Value>) -> Self {
        Self { items }
    }

    pub fn items(&self) -> &[crate::value::Value] {
        &self.items
    }
}

#[callable_impl]
impl ArrayContext {
    #[docs(
        name = "array.length",
        description = "Length of the array",
        example = "print my_array.length"
    )]
    #[property]
    pub fn length(&self) -> f64 {
        self.items.len() as f64
    }

    #[docs(
        name = "array.first",
        description = "Returns the first element of the array",
        example = "print my_array.first"
    )]
    #[method]
    pub fn first(&self) -> String {
        self.items
            .first()
            .map(|v| format!("{:?}", v))
            .unwrap_or_else(|| "null".to_string())
    }

    #[docs(
        name = "array.last",
        description = "Returns the last element of the array",
        example = "print my_array.last"
    )]
    #[method]
    pub fn last(&self) -> String {
        self.items
            .last()
            .map(|v| format!("{:?}", v))
            .unwrap_or_else(|| "null".to_string())
    }

    #[docs(
        name = "array.is_empty",
        description = "Checks if the array is empty",
        example = "if my_array.is_empty { print \"Array is empty\" }"
    )]
    #[method]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    #[docs(
        name = "array.sum",
        description = "Calculates the sum of numeric elements in the array",
        example = "print my_array.sum"
    )]
    #[method]
    pub fn sum(&self) -> f64 {
        use crate::value::Value;
        self.items
            .iter()
            .filter_map(|v| match v {
                Value::Number(n) => Some(*n),
                _ => None,
            })
            .sum()
    }
}

impl ArrayContext {
    pub fn filter(
        &self,
        executor: &crate::executor::Executor,
        param: &str,
        body: &githook_syntax::ast::Expression,
    ) -> Result<crate::value::Value> {
        use crate::value::Value;
        let mut result = Vec::new();
        for item in &self.items {
            let mut scoped_executor = executor.clone();
            scoped_executor.set_variable(param.to_string(), item.clone());
            let predicate_result = scoped_executor.eval_expression(body)?;
            if predicate_result.is_truthy() {
                result.push(item.clone());
            }
        }
        Ok(Value::Array(result))
    }

    pub fn map(
        &self,
        executor: &crate::executor::Executor,
        param: &str,
        body: &githook_syntax::ast::Expression,
    ) -> Result<crate::value::Value> {
        use crate::value::Value;
        let mut result = Vec::new();
        for item in &self.items {
            let mut scoped_executor = executor.clone();
            scoped_executor.set_variable(param.to_string(), item.clone());
            let mapped_value = scoped_executor.eval_expression(body)?;
            result.push(mapped_value);
        }
        Ok(Value::Array(result))
    }

    pub fn find(
        &self,
        executor: &crate::executor::Executor,
        param: &str,
        body: &githook_syntax::ast::Expression,
    ) -> Result<crate::value::Value> {
        use crate::value::Value;
        for item in &self.items {
            let mut scoped_executor = executor.clone();
            scoped_executor.set_variable(param.to_string(), item.clone());
            let predicate_result = scoped_executor.eval_expression(body)?;
            if predicate_result.is_truthy() {
                return Ok(item.clone());
            }
        }
        Ok(Value::Null)
    }

    pub fn any(
        &self,
        executor: &crate::executor::Executor,
        param: &str,
        body: &githook_syntax::ast::Expression,
    ) -> Result<crate::value::Value> {
        use crate::value::Value;
        for item in &self.items {
            let mut scoped_executor = executor.clone();
            scoped_executor.set_variable(param.to_string(), item.clone());
            let predicate_result = scoped_executor.eval_expression(body)?;
            if predicate_result.is_truthy() {
                return Ok(Value::Bool(true));
            }
        }
        Ok(Value::Bool(false))
    }

    pub fn all(
        &self,
        executor: &crate::executor::Executor,
        param: &str,
        body: &githook_syntax::ast::Expression,
    ) -> Result<crate::value::Value> {
        use crate::value::Value;
        for item in &self.items {
            let mut scoped_executor = executor.clone();
            scoped_executor.set_variable(param.to_string(), item.clone());
            let predicate_result = scoped_executor.eval_expression(body)?;
            if !predicate_result.is_truthy() {
                return Ok(Value::Bool(false));
            }
        }
        Ok(Value::Bool(true))
    }
}

#[derive(Debug, Clone, Default)]
pub struct HttpContext;

impl HttpContext {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Clone)]
pub struct HttpResponseContext {
    status: u16,
    body: String,
    headers: std::collections::HashMap<String, String>,
}

impl HttpResponseContext {
    pub fn new(
        status: u16,
        body: String,
        headers: std::collections::HashMap<String, String>,
    ) -> Self {
        Self {
            status,
            body,
            headers,
        }
    }
}

#[callable_impl]
impl HttpResponseContext {
    #[docs(
        name = "response.status",
        description = "HTTP status code",
        example = "if response.status == 200 { print \"OK\" }"
    )]
    #[property]
    pub fn status(&self) -> f64 {
        self.status as f64
    }

    #[docs(
        name = "response.body",
        description = "Response body as string",
        example = "print response.body"
    )]
    #[property]
    pub fn body(&self) -> String {
        self.body.clone()
    }

    #[docs(
        name = "response.ok",
        description = "Whether status is 2xx",
        example = "if response.ok { print \"Success\" }"
    )]
    #[property]
    pub fn ok(&self) -> bool {
        self.status >= 200 && self.status < 300
    }

    #[docs(
        name = "response.header",
        description = "Get response header by name",
        example = "print response.header(\"content-type\")"
    )]
    #[method]
    pub fn header(&self, name: &str) -> String {
        self.headers
            .get(&name.to_lowercase())
            .cloned()
            .unwrap_or_default()
    }
}

impl HttpResponseContext {
    pub fn json_parsed(&self) -> crate::value::Value {
        match serde_json::from_str::<serde_json::Value>(&self.body) {
            Ok(parsed) => json_to_value(parsed),
            Err(_) => crate::value::Value::Null,
        }
    }
}

fn json_to_value(json: serde_json::Value) -> crate::value::Value {
    use crate::value::Value;
    match json {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Number(i as f64)
            } else if let Some(f) = n.as_f64() {
                Value::Number(f)
            } else {
                Value::Null
            }
        }
        serde_json::Value::String(s) => Value::String(s),
        serde_json::Value::Array(arr) => Value::Array(arr.into_iter().map(json_to_value).collect()),
        serde_json::Value::Object(obj) => {
            let mut dict = crate::value::Object::new("Dict");
            for (k, v) in obj {
                dict.set(&k, json_to_value(v));
            }
            Value::Object(dict)
        }
    }
}
