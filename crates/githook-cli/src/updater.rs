use anyhow::{Context, Result};
use colored::*;
use std::env;
use std::fs;
use std::path::PathBuf;

pub fn check_for_updates() -> Result<()> {
    let current_version = env!("CARGO_PKG_VERSION");

    println!("Checking for updates...");
    println!("Current version: {}", current_version.cyan());

    match get_latest_version() {
        Ok(latest) => {
            println!("Latest version:  {}", latest.cyan());

            if is_newer_version(&latest, current_version) {
                println!("\n{} New version available!", "!".yellow());
                println!("To update, run: {} githook --update", ">".cyan());
            } else {
                println!("\n{} You already have the latest version.", "o".green());
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("\n{} Could not check for updates: {}", "x".red(), e);
            eprintln!("Check manually: https://github.com/scholzdev/githook/releases");
            Err(e)
        }
    }
}

pub fn perform_update() -> Result<()> {
    let current_version = env!("CARGO_PKG_VERSION");

    println!("{} Checking for updates...\n", "-".cyan());

    match get_latest_version() {
        Ok(latest) => {
            println!("Current version: {}", current_version.cyan());
            println!("Latest version:  {}", latest.cyan());

            if is_newer_version(&latest, current_version) {
                println!("\n{} New version available!", "!".yellow());
                println!("{}  Downloading and installing...\n", "-".cyan());

                let backup_path = get_backup_path()?;
                if let Some(parent) = backup_path.parent() {
                    fs::create_dir_all(parent).context("Failed to create backup directory")?;
                }

                let current_exe =
                    env::current_exe().context("Failed to locate running executable")?;

                fs::copy(&current_exe, &backup_path).context("Failed to create backup")?;

                println!(
                    "{}  Created backup at: {}",
                    "-".dimmed(),
                    backup_path.display()
                );

                if let Err(e) = install_update(&latest) {
                    eprintln!("{} Update failed: {}", "x".red(), e);
                    eprintln!("Install manually: https://github.com/scholzdev/githook/releases");
                    return Err(e);
                }

                println!(
                    "{} Successfully updated to version {}!\n",
                    "o".green(),
                    latest
                );
                println!("{} Restart githook to use the new version.", "-".dimmed());
            } else {
                println!("{} You already have the latest version.\n", "o".green());
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("{} Could not check for updates: {}", "x".red(), e);
            eprintln!("Check manually: https://github.com/scholzdev/githook/releases\n");
            Err(e)
        }
    }
}

fn get_latest_version() -> Result<String> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .get("https://api.github.com/repos/scholzdev/githook/releases/latest")
        .header("User-Agent", "githook")
        .send()?;

    if response.status().is_success() {
        let json: serde_json::Value = response.json()?;
        if let Some(tag) = json["tag_name"].as_str() {
            return Ok(tag.trim_start_matches('v').to_string());
        }
    }

    Err(anyhow::anyhow!(
        "Could not parse latest version from GitHub"
    ))
}

fn is_newer_version(latest: &str, current: &str) -> bool {
    let parse_version = |v: &str| -> (u32, u32, u32) {
        let parts: Vec<&str> = v.split('.').collect();
        (
            parts.first().and_then(|p| p.parse().ok()).unwrap_or(0),
            parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(0),
            parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(0),
        )
    };

    let (latest_major, latest_minor, latest_patch) = parse_version(latest);
    let (current_major, current_minor, current_patch) = parse_version(current);

    if latest_major != current_major {
        latest_major > current_major
    } else if latest_minor != current_minor {
        latest_minor > current_minor
    } else {
        latest_patch > current_patch
    }
}

fn install_update(version: &str) -> Result<()> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    let target = match (os, arch) {
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        _ => return Err(anyhow::anyhow!("Unsupported platform: {} {}", os, arch)),
    };

    let asset_name = if cfg!(windows) {
        format!("githook-{}.exe", target)
    } else {
        format!("githook-{}", target)
    };

    let download_url = format!(
        "https://github.com/scholzdev/githook/releases/download/v{}/{}",
        version, asset_name
    );

    println!("{}  Downloading from: {}", "-".dimmed(), download_url);

    let client = reqwest::blocking::Client::new();
    let response = client
        .get(&download_url)
        .header("User-Agent", "githook")
        .send()
        .context("Failed to download update")?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Download failed with status: {} (URL: {})",
            response.status(),
            download_url
        ));
    }

    let binary_data = response.bytes().context("Failed to read binary data")?;

    println!("{}  Downloaded {} bytes", "-".dimmed(), binary_data.len());

    let temp_file = tempfile::NamedTempFile::new().context("Failed to create temporary file")?;

    fs::write(temp_file.path(), &binary_data).context("Failed to write temporary file")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(temp_file.path())?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(temp_file.path(), perms)?;
    }

    println!("{}  Installing new version...", "-".dimmed());

    self_update::Move::from_source(temp_file.path())
        .replace_using_temp(&get_backup_path()?)
        .to_dest(&env::current_exe()?)
        .context("Failed to replace executable")?;

    Ok(())
}

fn get_backup_path() -> Result<PathBuf> {
    let cache_dir =
        dirs::cache_dir().ok_or_else(|| anyhow::anyhow!("Could not determine cache directory"))?;

    Ok(cache_dir.join("githook").join("githook_backup"))
}
