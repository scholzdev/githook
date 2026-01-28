use std::path::PathBuf;
use std::fs;
use anyhow::{Result, bail, anyhow};

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
    let path = resolve_package_path(namespace, name)?;

    if path.exists() {
        return Ok(fs::read_to_string(&path)?);
    }

    if namespace != "local" {
        eprintln!("Package @{}/{} not found locally. Try installing it first.", namespace, name);
        eprintln!("Attempting to fetch from default repository...");
        
        let repo_url = get_default_repo_url(namespace);
        
        validate_repo_url(&repo_url)?;
        
        let url = format!(
            "https://raw.githubusercontent.com/{}/refs/heads/main/{}/{}/{}.ghook",
            repo_url, namespace, name, name
        );
        
        eprintln!("Fetching from: {}", url);
        
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        
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
        
        let content = response.text()?;
        
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, &content)?;
        
        eprintln!("Package @{}/{} cached successfully!", namespace, name);
        
        return Ok(content);
    }

    bail!(
        "Package not found: @{}/{}",
        namespace,
        name
    );
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
    "scholzdev/githook-packages".to_string()
}