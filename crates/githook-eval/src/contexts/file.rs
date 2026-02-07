//! File and path context types.

use githook_macros::{callable_impl, docs};
use std::{
    fmt,
    path::{Path, PathBuf},
};

/// Typed context for path operations (`name`, `extension`, `directory`, etc.).
#[derive(Debug, Clone)]
pub struct PathContext {
    path_buf: PathBuf,
}

impl PathContext {
    /// Creates a new path context from any path-like value.
    pub fn from_path(path: impl AsRef<Path>) -> Self {
        Self {
            path_buf: path.as_ref().to_path_buf(),
        }
    }

    /// Returns the path as a string slice.
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
        name = "path.join",
        description = "Joins path components",
        example = "file.path.join(\"subdir\")"
    )]
    #[method]
    pub fn join(&self, other: &str) -> String {
        self.path_buf.join(other).to_string_lossy().to_string()
    }
}

/// Typed context for file operations (`name`, `content`, `exists`, etc.).
///
/// Backs `File` objects returned by `git.files.staged` and the `file()` built-in.
#[derive(Debug, Clone)]
pub struct FileContext {
    /// The underlying path context.
    pub path: PathContext,
    path_buf: PathBuf,
}

impl FileContext {
    /// Creates a file context from a filesystem path.
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
        name = "file.exists",
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
        description = "Checks if the file exists and is accessible",
        example = "if file.is_readable { print \"File is accessible\" }"
    )]
    #[method]
    pub fn is_readable(&self) -> bool {
        self.path_buf.metadata().is_ok()
    }

    #[docs(
        name = "file.is_executable",
        description = "Checks if the file is executable",
        example = "if file.is_executable { print \"File is executable\" }"
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
        name = "file.contains",
        description = "Checks if the file path contains a substring",
        example = "if file.contains(\"src\") { print \"In src directory\" }"
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
