use crate::ast::Statement;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone)]
struct CacheEntry {
    statements: Vec<Statement>,
    mtime: SystemTime,
    size: u64,
    hash: Option<u64>,
}

pub struct ParseCache {
    cache: std::sync::RwLock<HashMap<PathBuf, CacheEntry>>,
    max_entries: usize,
}

impl ParseCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            cache: std::sync::RwLock::new(HashMap::new()),
            max_entries,
        }
    }

    pub fn with_default_size() -> Self {
        let max_entries = std::env::var("GITHOOK_PARSE_CACHE_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(50);
        Self::new(max_entries)
    }

    pub fn get(&self, path: &Path) -> Option<Vec<Statement>> {
        let cache = self.cache.read().ok()?;
        let entry = cache.get(path)?;

        let metadata = std::fs::metadata(path).ok()?;
        let current_mtime = metadata.modified().ok()?;
        let current_size = metadata.len();

        if entry.size != current_size || entry.mtime != current_mtime {
            return None;
        }

        if let Some(cached_hash) = entry.hash {
            if let Ok(content) = std::fs::read_to_string(path) {
                let current_hash = Self::hash_content(&content);
                if current_hash != cached_hash {
                    return None;
                }
            }
        }

        Some(entry.statements.clone())
    }

    pub fn insert(&self, path: PathBuf, statements: Vec<Statement>, content: Option<&str>) {
        let Ok(metadata) = std::fs::metadata(&path) else {
            return;
        };
        let Ok(mtime) = metadata.modified() else {
            return;
        };
        let size = metadata.len();

        let hash = content.map(Self::hash_content);

        let entry = CacheEntry {
            statements,
            mtime,
            size,
            hash,
        };

        if let Ok(mut cache) = self.cache.write() {
            if cache.len() >= self.max_entries {
                if let Some(oldest_key) = cache.keys().next().cloned() {
                    cache.remove(&oldest_key);
                }
            }
            cache.insert(path, entry);
        }
    }

    pub fn clear(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
    }

    pub fn invalidate(&self, path: &Path) {
        if let Ok(mut cache) = self.cache.write() {
            cache.remove(path);
        }
    }

    pub fn stats(&self) -> CacheStats {
        let cache = self.cache.read().ok();
        CacheStats {
            entries: cache.as_ref().map(|c| c.len()).unwrap_or(0),
            max_entries: self.max_entries,
        }
    }

    fn hash_content(content: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }
}

impl Default for ParseCache {
    fn default() -> Self {
        Self::with_default_size()
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entries: usize,
    pub max_entries: usize,
}
