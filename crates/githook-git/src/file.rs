use anyhow::Result;
use std::{os::unix::fs::PermissionsExt, path::{Path, PathBuf}, fmt};
use githook_macros::callable_impl;

// ============================================================================
// PATH CONTEXT - Path information and operations
// ============================================================================

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
    #[property]
    pub fn string(&self) -> String {
        self.path_buf.to_string_lossy().to_string()
    }
    
    #[property]
    pub fn basename(&self) -> String {
        self.path_buf
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string()
    }
    
    #[property]
    pub fn extension(&self) -> String {
        self.path_buf
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e))
            .unwrap_or_default()
    }
    
    #[property]
    pub fn parent(&self) -> String {
        self.path_buf
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("")
            .to_string()
    }
    
    #[property]
    pub fn filename(&self) -> String {
        self.path_buf
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string()
    }
    
    #[method]
    pub fn join(&self, other: &str) -> String {
        self.path_buf.join(other).to_string_lossy().to_string()
    }
}

// ============================================================================
// FILE CONTEXT - File information for hooks
// ============================================================================

#[derive(Debug, Clone)]
pub struct FileContext {
    pub path: PathContext,
    path_buf: PathBuf,
}

impl FileContext {
    /// Create a FileContext from a path
    pub fn from_path(path: impl AsRef<Path>) -> Self {
        let path_buf = path.as_ref().to_path_buf();
        let path_ctx = PathContext::from_path(&path_buf);
        Self { 
            path: path_ctx,
            path_buf,
        }
    }
    
    /// Get file content
    pub fn content(&self) -> Result<String> {
        Ok(std::fs::read_to_string(&self.path_buf)?)
    }
}

// ============================================================================
// CALLABLE METHODS - Automatically exposed to GitHook scripts
// ============================================================================

#[callable_impl]
impl FileContext {
    #[property]
    pub fn name(&self) -> String {
        self.path_buf
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string()
    }
    
    #[property]
    pub fn basename(&self) -> String {
        self.path_buf
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string()
    }
    
    #[property]
    pub fn extension(&self) -> String {
        self.path_buf
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e))
            .unwrap_or_default()
    }
    
    #[property]
    pub fn dirname(&self) -> String {
        self.path_buf
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("")
            .to_string()
    }
    
    #[property]
    pub fn size(&self) -> f64 {
        std::fs::metadata(&self.path_buf)
            .map(|m| m.len() as f64)
            .unwrap_or(0.0)
    }

    #[method]
    pub fn exists(&self) -> bool {
        self.path_buf.exists()
    }
    
    #[method]
    pub fn is_file(&self) -> bool {
        self.path_buf.is_file()
    }
    
    #[method]
    pub fn is_dir(&self) -> bool {
        self.path_buf.is_dir()
    }

    #[method]
    pub fn is_readable(&self) -> bool {
        self.path_buf.metadata().is_ok()
    }
    
    #[method]
    pub fn is_executable(&self) -> bool {
        if let Ok(metadata) = std::fs::metadata(&self.path_buf) {
            metadata.permissions().mode() & 0o111 != 0
        } else {
            false
        }
    }
    
    #[method]
    pub fn is_symlink(&self) -> bool {
        self.path_buf.is_symlink()
    }
    
    #[method]
    pub fn is_absolute(&self) -> bool {
        self.path_buf.is_absolute()
    }
    
    #[method]
    pub fn is_relative(&self) -> bool {
        self.path_buf.is_relative()
    }

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

    #[method]
    pub fn is_hidden(&self) -> bool {
        self.path_buf
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name_str| name_str.starts_with('.'))
            .unwrap_or(false)
    }

    #[method]
    pub fn permissions(&self) -> u32 {
        self.path_buf
        .metadata()
            .map(|m| m.permissions().mode())
            .unwrap_or(0)
    }

    #[method]
    pub fn test(&self) -> bool {
        return true
    }
    
    #[method]
    pub fn contains(&self, pattern: &str) -> bool {
        self.path.string().contains(pattern)
    }
    
    #[method]
    pub fn starts_with(&self, prefix: &str) -> bool {
        self.path.string().starts_with(prefix)
    }
    
    #[method]
    pub fn ends_with(&self, suffix: &str) -> bool {
        self.path.string().ends_with(suffix)
    }
}
