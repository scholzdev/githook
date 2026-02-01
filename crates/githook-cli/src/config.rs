use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default = "default_true")]
    pub colored: bool,

    #[serde(default)]
    pub verbose: bool,

    #[serde(default = "default_true")]
    pub cache: bool,

    #[serde(default)]
    pub only_groups: Vec<String>,

    #[serde(default)]
    pub skip_groups: Vec<String>,

    #[serde(default = "default_search_paths")]
    pub search_paths: Vec<String>,
    
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
    
    #[serde(default)]
    pub timeout: u64,
}

fn default_true() -> bool {
    true
}

fn default_search_paths() -> Vec<String> {
    vec![
        ".githook".to_string(),
        ".git/hooks".to_string(),
        ".".to_string(),
    ]
}

impl Config {
    /// Load config from .githookrc (TOML format)
    pub fn load() -> Result<Self> {
        let config_paths = vec![
            PathBuf::from(".githookrc"),
            PathBuf::from(".githookrc.toml"),
            PathBuf::from(".config/githookrc"),
        ];
        
        for path in config_paths {
            if path.exists() {
                return Self::load_from_file(&path);
            }
        }
        
        // No config found, use defaults
        Ok(Config::default())
    }
    
    /// Load config from specific file
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
    
    /// Create a default config file
    #[allow(dead_code)]
    pub fn create_default(path: &Path) -> Result<()> {
        let default_config = r#"# githook configuration file
# See https://githook.dev/configuration for details

# Enable colored output in terminal
colored = true

# Show verbose execution details
verbose = false

# Enable package caching
cache = true

# Only run specific groups (empty = all groups)
only_groups = []

# Skip specific groups
skip_groups = []

# Paths to search for hook files
search_paths = [".githook", ".git/hooks", "."]

# Custom environment variables
[env]
# RUST_LOG = "debug"

# Command timeout in seconds (0 = no timeout)
timeout = 300
"#;
        
        fs::write(path, default_config)?;
        Ok(())
    }
    
    /// Merge CLI arguments into config
    pub fn merge_cli_args(
        &mut self,
        cache: Option<bool>,
        verbose: bool,
        only_groups: Option<String>,
        skip_groups: Option<String>,
    ) {
        if let Some(cache) = cache {
            self.cache = cache;
        }
        
        if verbose {
            self.verbose = true;
        }
        
        if let Some(groups) = only_groups {
            self.only_groups = groups.split(',').map(|s| s.trim().to_string()).collect();
        }
        
        if let Some(groups) = skip_groups {
            self.skip_groups = groups.split(',').map(|s| s.trim().to_string()).collect();
        }
    }
}
