//! Runtime configuration for the Githook executor.
//!
//! [`Config`] controls timeouts, parallelism, and HTTP authentication.
//! Use [`Config::default()`] for sensible defaults (30 s command timeout,
//! system thread count, no auth token).
//!
//! # Config file: `.ghrc`
//!
//! Similar to `.gitconfig`, Githook uses `.ghrc` files with TOML syntax:
//!
//! - **Global**: `~/.ghrc` — applies to all projects
//! - **Local**: `.ghrc` or `.githook/.ghrc` in your project — overrides global
//!
//! ```toml
//! # .ghrc example
//! command_timeout = 60
//! http_timeout = 10
//! max_parallel_threads = 4
//! 
//! # Package repository configuration
//! package_remote_url = "yourorg/githook-packages"
//! package_remote_type = "github"  # or "gitlab"
//! package_access_token = "ghp_your_token"  # for private repos
//! ```
//!
//! All fields are optional. Local values override global values.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// TOML-friendly intermediate representation (all fields optional).
#[derive(Debug, Deserialize, Default)]
struct ConfigFile {
    /// Command timeout in seconds.
    command_timeout: Option<u64>,
    /// HTTP timeout in seconds.
    http_timeout: Option<u64>,
    /// Max parallel threads (0 = all cores).
    max_parallel_threads: Option<usize>,
    /// Bearer token for HTTP requests.
    auth_token: Option<String>,
    /// Package repository URL (e.g., "owner/repo" for GitHub, "gitlab.com/owner/repo" for GitLab).
    package_remote_url: Option<String>,
    /// Package repository type ("github" or "gitlab").
    package_remote_type: Option<String>,
    /// Access token for private package repositories.
    package_access_token: Option<String>,
}

/// Runtime configuration for the executor.
///
/// # Defaults
///
/// | Setting | Default |
/// |---------|---------|  
/// | `command_timeout` | 30 s |
/// | `http_timeout` | 30 s |
/// | `max_parallel_threads` | `0` (= use all available cores) |
/// | `auth_token` | `None` |
/// | `package_remote_url` | `"scholzdev/githooks-packages"` |
/// | `package_remote_type` | `"github"` |
/// | `package_access_token` | `None` |
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    /// Maximum time a `run` / `exec()` command is allowed to run.
    pub command_timeout: Duration,
    /// Timeout for HTTP requests (`http.get`, `http.post`, …).
    pub http_timeout: Duration,
    /// Maximum number of threads for `parallel { … }` blocks.
    /// `0` means "use all available cores" (rayon default).
    pub max_parallel_threads: usize,
    /// Optional bearer token sent as `Authorization: Bearer <token>`
    /// with every HTTP request (useful for private package registries).
    pub auth_token: Option<String>,
    /// Package repository URL (e.g., "owner/repo" for GitHub, "gitlab.com/owner/repo" for GitLab).
    pub package_remote_url: String,
    /// Package repository type: "github" or "gitlab".
    pub package_remote_type: String,
    /// Access token for private package repositories (GitHub PAT or GitLab token).
    pub package_access_token: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            command_timeout: Duration::from_secs(30),
            http_timeout: Duration::from_secs(30),
            max_parallel_threads: 0,
            auth_token: None,
            package_remote_url: "scholzdev/githooks-packages".to_string(),
            package_remote_type: "github".to_string(),
            package_access_token: None,
        }
    }
}

impl Config {
    /// Creates a new config with all defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads configuration by merging global and local `.ghrc` files.
    ///
    /// 1. Loads `~/.ghrc` (global) if it exists
    /// 2. Searches for `.ghrc` or `.githook/.ghrc` starting from `start_dir`
    /// 3. Local values override global values
    ///
    /// Returns `Config::default()` if no config files are found.
    pub fn load(start_dir: impl AsRef<Path>) -> Result<Self> {
        let mut config = Self::default();

        // Load global config first
        if let Some(global_path) = Self::find_global_config() {
            if let Ok(global_config) = Self::from_file(&global_path) {
                config = global_config;
            }
        }

        // Load local config and merge (local overrides global)
        if let Some(local_path) = Self::find_local_config(start_dir) {
            let local_file_content = std::fs::read_to_string(&local_path)
                .with_context(|| format!("Failed to read config file: {}", local_path.display()))?;
            let local_file: ConfigFile = toml::from_str(&local_file_content)
                .with_context(|| format!("Failed to parse {}", local_path.display()))?;

            // Merge: local overrides global
            if let Some(timeout) = local_file.command_timeout {
                config.command_timeout = Duration::from_secs(timeout);
            }
            if let Some(timeout) = local_file.http_timeout {
                config.http_timeout = Duration::from_secs(timeout);
            }
            if let Some(threads) = local_file.max_parallel_threads {
                config.max_parallel_threads = threads;
            }
            if local_file.auth_token.is_some() {
                config.auth_token = local_file.auth_token;
            }
            if let Some(url) = local_file.package_remote_url {
                config.package_remote_url = url;
            }
            if let Some(t) = local_file.package_remote_type {
                config.package_remote_type = t;
            }
            if local_file.package_access_token.is_some() {
                config.package_access_token = local_file.package_access_token;
            }
        }

        Ok(config)
    }

    /// Loads configuration from a specific file.
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        Self::from_str(&content)
    }

    /// Parses configuration from a TOML string.
    pub fn from_str(toml_str: &str) -> Result<Self> {
        let file: ConfigFile =
            toml::from_str(toml_str).context("Failed to parse config")?;

        let defaults = Self::default();
        Ok(Self {
            command_timeout: file
                .command_timeout
                .map(Duration::from_secs)
                .unwrap_or(defaults.command_timeout),
            http_timeout: file
                .http_timeout
                .map(Duration::from_secs)
                .unwrap_or(defaults.http_timeout),
            max_parallel_threads: file
                .max_parallel_threads
                .unwrap_or(defaults.max_parallel_threads),
            auth_token: file.auth_token.or(defaults.auth_token),
            package_remote_url: file.package_remote_url.unwrap_or(defaults.package_remote_url),
            package_remote_type: file.package_remote_type.unwrap_or(defaults.package_remote_type),
            package_access_token: file.package_access_token.or(defaults.package_access_token),
        })
    }

    /// Finds the global config file at `~/.ghrc`.
    fn find_global_config() -> Option<PathBuf> {
        dirs::home_dir().map(|home| home.join(".ghrc")).filter(|p| p.is_file())
    }

    /// Walks up from `start_dir` looking for `.ghrc` or `.githook/.ghrc`.
    fn find_local_config(start_dir: impl AsRef<Path>) -> Option<PathBuf> {
        let mut dir = start_dir.as_ref().to_path_buf();

        // Make sure we have an absolute path
        if let Ok(abs) = dir.canonicalize() {
            dir = abs;
        }

        loop {
            // Check for .ghrc in current directory
            let rc_file = dir.join(".ghrc");
            if rc_file.is_file() {
                return Some(rc_file);
            }

            // Check for .githook/.ghrc as fallback
            let githook_rc = dir.join(".githook").join(".ghrc");
            if githook_rc.is_file() {
                return Some(githook_rc);
            }

            // Walk up
            if !dir.pop() {
                return None;
            }
        }
    }

    /// Builder: set the command timeout.
    pub fn with_command_timeout(mut self, timeout: Duration) -> Self {
        self.command_timeout = timeout;
        self
    }

    /// Builder: set the HTTP timeout.
    pub fn with_http_timeout(mut self, timeout: Duration) -> Self {
        self.http_timeout = timeout;
        self
    }

    /// Builder: limit parallel threads (`0` = all cores).
    pub fn with_max_parallel_threads(mut self, n: usize) -> Self {
        self.max_parallel_threads = n;
        self
    }

    /// Builder: set the auth token for HTTP requests.
    pub fn with_auth_token(mut self, token: impl Into<String>) -> Self {
        self.auth_token = Some(token.into());
        self
    }

    /// Builder: set package remote URL (e.g., "owner/repo" or "gitlab.com/owner/repo").
    pub fn with_package_remote_url(mut self, url: impl Into<String>) -> Self {
        self.package_remote_url = url.into();
        self
    }

    /// Builder: set package remote type ("github" or "gitlab").
    pub fn with_package_remote_type(mut self, remote_type: impl Into<String>) -> Self {
        self.package_remote_type = remote_type.into();
        self
    }

    /// Builder: set package access token.
    pub fn with_package_access_token(mut self, token: impl Into<String>) -> Self {
        self.package_access_token = Some(token.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = Config::default();
        assert_eq!(cfg.command_timeout, Duration::from_secs(30));
        assert_eq!(cfg.http_timeout, Duration::from_secs(30));
        assert_eq!(cfg.max_parallel_threads, 0);
        assert!(cfg.auth_token.is_none());
        assert_eq!(cfg.package_remote_url, "scholzdev/githooks-packages");
        assert_eq!(cfg.package_remote_type, "github");
        assert!(cfg.package_access_token.is_none());
    }

    #[test]
    fn test_parse_full_config() {
        let toml = r#"
            command_timeout = 60
            http_timeout = 10
            max_parallel_threads = 4
            auth_token = "secret"
        "#;
        let cfg = Config::from_str(toml).unwrap();
        assert_eq!(cfg.command_timeout, Duration::from_secs(60));
        assert_eq!(cfg.http_timeout, Duration::from_secs(10));
        assert_eq!(cfg.max_parallel_threads, 4);
        assert_eq!(cfg.auth_token.as_deref(), Some("secret"));
    }

    #[test]
    fn test_parse_partial_config() {
        let toml = r#"
            command_timeout = 120
        "#;
        let cfg = Config::from_str(toml).unwrap();
        assert_eq!(cfg.command_timeout, Duration::from_secs(120));
        // rest stays default
        assert_eq!(cfg.http_timeout, Duration::from_secs(30));
        assert_eq!(cfg.max_parallel_threads, 0);
        assert!(cfg.auth_token.is_none());
    }

    #[test]
    fn test_parse_empty_config() {
        let cfg = Config::from_str("").unwrap();
        assert_eq!(cfg, Config::default());
    }

    #[test]
    fn test_merge_configs() {
        // Simulate global config
        let global = r#"
            command_timeout = 60
            http_timeout = 10
            max_parallel_threads = 4
        "#;
        let mut cfg = Config::from_str(global).unwrap();

        // Simulate local override
        let local = r#"
            command_timeout = 120
            auth_token = "local_token"
        "#;
        let local_file: ConfigFile = toml::from_str(local).unwrap();

        // Merge local into global
        if let Some(t) = local_file.command_timeout {
            cfg.command_timeout = Duration::from_secs(t);
        }
        if local_file.auth_token.is_some() {
            cfg.auth_token = local_file.auth_token;
        }

        // Check merged result
        assert_eq!(cfg.command_timeout, Duration::from_secs(120)); // overridden
        assert_eq!(cfg.http_timeout, Duration::from_secs(10)); // from global
        assert_eq!(cfg.max_parallel_threads, 4); // from global
        assert_eq!(cfg.auth_token.as_deref(), Some("local_token")); // added by local
    }

    #[test]
    fn test_builder_methods() {
        let cfg = Config::new()
            .with_command_timeout(Duration::from_secs(45))
            .with_http_timeout(Duration::from_secs(5))
            .with_max_parallel_threads(2)
            .with_auth_token("tok");
        assert_eq!(cfg.command_timeout, Duration::from_secs(45));
        assert_eq!(cfg.http_timeout, Duration::from_secs(5));
        assert_eq!(cfg.max_parallel_threads, 2);
        assert_eq!(cfg.auth_token.as_deref(), Some("tok"));
    }

    #[test]
    fn test_package_config() {
        let toml = r#"
            package_remote_url = "myorg/private-hooks"
            package_remote_type = "gitlab"
            package_access_token = "glpat-secret"
        "#;
        let cfg = Config::from_str(toml).unwrap();
        assert_eq!(cfg.package_remote_url, "myorg/private-hooks");
        assert_eq!(cfg.package_remote_type, "gitlab");
        assert_eq!(cfg.package_access_token.as_deref(), Some("glpat-secret"));
    }

    #[test]
    fn test_package_builder() {
        let cfg = Config::new()
            .with_package_remote_url("custom/repo")
            .with_package_remote_type("github")
            .with_package_access_token("ghp_token");
        assert_eq!(cfg.package_remote_url, "custom/repo");
        assert_eq!(cfg.package_remote_type, "github");
        assert_eq!(cfg.package_access_token.as_deref(), Some("ghp_token"));
    }
}