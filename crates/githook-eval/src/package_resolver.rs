use std::path::PathBuf;
use std::fs;
use anyhow::{Result, bail, anyhow};
use std::sync::Mutex;
use lru::LruCache;
use std::num::NonZeroUsize;
use once_cell::sync::Lazy;

// Global LRU cache for package contents (50 packages max)
static PACKAGE_CACHE: Lazy<Mutex<LruCache<String, String>>> = Lazy::new(|| {
    Mutex::new(LruCache::new(NonZeroUsize::new(50).unwrap()))
});

fn local_packages_dir() -> Result<PathBuf> {
    let home = dirs::home_dir()
        .ok_or_else(|| anyhow!("Could not determine home directory"))?;
    Ok(home.join(".githook").join("packages"))
}

fn cache_packages_dir() -> Result<PathBuf> {
    match dirs::cache_dir() {
        Some(cache) => Ok(cache.join("githook").join("packages")),
        None => bail!("Could not determine cache directory"),
    }
}

fn validate_package_identifier(identifier: &str) -> Result<()> {
    if identifier.is_empty() {
        bail!("Package identifier cannot be empty");
    }
    
    if !identifier.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        bail!("Invalid package identifier '{}': only alphanumeric, '-', and '_' allowed", identifier);
    }
    
    if identifier.contains("..") || identifier.contains('/') || identifier.contains('\\') {
        bail!("Invalid package identifier '{}': path traversal not allowed", identifier);
    }
    
    if identifier.len() > 100 {
        bail!("Package identifier too long (max 100 characters)");
    }
    
    Ok(())
}

pub fn resolve_package_path(namespace: &str, name: &str) -> Result<PathBuf> {
    validate_package_identifier(namespace)?;
    validate_package_identifier(name)?;
    
    let base_dir = match namespace {
        "local" => local_packages_dir()?,
        _ => cache_packages_dir()?,
    };

    let path = base_dir
        .join(namespace)
        .join(name)
        .join(format!("{}.ghook", name));

    Ok(path)
}

fn validate_repo_url(repo_url: &str) -> Result<()> {
    if !repo_url.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '/') {
        bail!("Invalid repository URL format: '{}'", repo_url);
    }
    
    let parts: Vec<&str> = repo_url.split('/').collect();
    if parts.len() != 2 {
        bail!("Repository URL must be in format 'owner/repo', got: '{}'", repo_url);
    }
    
    if parts[0].is_empty() || parts[1].is_empty() {
        bail!("Repository owner and name cannot be empty");
    }
    
    if parts[0].len() > 100 || parts[1].len() > 100 {
        bail!("Repository owner or name too long (max 100 characters each)");
    }
    
    Ok(())
}

pub fn load_package(
    namespace: &str,
    name: &str,
) -> Result<String> {
    // Check LRU cache first
    let cache_key = format!("{}::{}", namespace, name);
    
    if let Ok(mut cache) = PACKAGE_CACHE.lock() {
        if let Some(cached_content) = cache.get(&cache_key) {
            return Ok(cached_content.clone());
        }
    }
    
    let path = resolve_package_path(namespace, name)?;
    let etag_path = path.with_extension("etag");

    let content = if namespace == "local" {
        if path.exists() {
            fs::read_to_string(&path)?
        } else {
            bail!("Package not found: @{}/{} (local namespace only checks filesystem)", namespace, name);
        }
    } else {
        let repo_url = get_default_repo_url(namespace);
        validate_repo_url(&repo_url)?;
        
        let url = format!(
            "https://raw.githubusercontent.com/{}/refs/heads/main/{}/{}/{}.ghook",
            repo_url, namespace, name, name
        );

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        if path.exists() && etag_path.exists() {
            let cached_etag = fs::read_to_string(&etag_path).ok();
            
            if let Some(etag) = cached_etag {
                let response = client.get(&url)
                    .header("If-None-Match", etag.trim())
                    .send()?;
                
                if response.status() == 304 {
                    if cfg!(debug_assertions) {
                        eprintln!("✓ Package @{}/{} is up-to-date (using cache)", namespace, name);
                    }
                    fs::read_to_string(&path)?
                } else if response.status().is_success() {
                    let new_etag = response.headers()
                        .get("etag")
                        .and_then(|v| v.to_str().ok())
                        .map(|s| s.to_string());
                
                    let content = response.text()?;
                    
                    if let Some(parent) = path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&path, &content)?;
                    
                    if let Some(tag) = new_etag {
                        let _ = fs::write(&etag_path, tag);
                    }
                    
                    eprintln!("✓ Package @{}/{} updated", namespace, name);
                    content
                } else {
                    bail!("Failed to fetch package: HTTP {}", response.status());
                }
            } else {
                // No cached etag, fetch fresh
                let response = client.get(&url).send()?;
                
                if !response.status().is_success() {
                    bail!(
                        "Failed to fetch package @{}/{} from {}: HTTP {}",
                        namespace,
                        name,
                        url,
                        response.status()
                    );
                }
                
                let etag = response.headers()
                    .get("etag")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());
                
                let content = response.text()?;
                
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&path, &content)?;
                
                if let Some(tag) = etag {
                    let _ = fs::write(&etag_path, tag);
                }
                
                eprintln!("✓ Package @{}/{} cached successfully", namespace, name);
                content
            }
        } else {
            // No cache exists, fetch fresh
            eprintln!("Fetching package @{}/{}...", namespace, name);
            
            let response = client.get(&url).send()?;
            
            if !response.status().is_success() {
                bail!(
                    "Failed to fetch package @{}/{} from {}: HTTP {}",
                    namespace,
                    name,
                    url,
                    response.status()
                );
            }
            
            let etag = response.headers()
                .get("etag")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            
            let content = response.text()?;
            
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, &content)?;
            
            if let Some(tag) = etag {
                let _ = fs::write(&etag_path, tag);
            }
            
            eprintln!("✓ Package @{}/{} cached successfully", namespace, name);
            content
        }
    };
    
    // Store in LRU cache
    if let Ok(mut cache) = PACKAGE_CACHE.lock() {
        cache.put(cache_key, content.clone());
    }
    
    Ok(content)
}

pub async fn load_or_fetch_package(
    namespace: &str,
    name: &str,
    repo_url: &str,
) -> Result<String> {
    let path = resolve_package_path(namespace, name)?;

    if path.exists() {
        return Ok(fs::read_to_string(&path)?);
    }

    if namespace == "local" {
        bail!(
            "Package not found: @{}/{} (local namespace only checks filesystem)",
            namespace,
            name
        );
    }

    validate_repo_url(repo_url)?;
    
    let url = format!(
        "https://raw.githubusercontent.com/{}/refs/heads/main/{}/{}/{}.ghook",
        repo_url, namespace, name, name
    );

    eprintln!("Fetching package from: {}", url);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    
    let response = client.get(&url).send().await?;

    if !response.status().is_success() {
        bail!(
            "Failed to fetch package @{}/{} from {}: HTTP {}",
            namespace,
            name,
            url,
            response.status()
        );
    }

    let content = response.text().await?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, &content)?;

    Ok(content)
}

pub fn get_default_repo_url(_namespace: &str) -> String {
    "scholzdev/githooks-packages".to_string()
}