use githook_syntax::error::Span;
use githook_syntax::{ast::Statement, lexer, parser};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

pub fn resolve_import_path(current_file_uri: &str, relative_path: &str) -> Option<PathBuf> {
    info!(
        "Resolving import: uri={}, relative_path={}",
        current_file_uri, relative_path
    );

    let current_path = uri_to_path(current_file_uri)?;
    info!("Current path: {:?}", current_path);

    let current_dir = current_path.parent()?;
    info!("Current dir: {:?}", current_dir);

    let clean_path = relative_path.strip_prefix("./").unwrap_or(relative_path);

    let resolved = current_dir.join(clean_path);
    info!("Resolved path: {:?}", resolved);

    Some(resolved)
}

fn uri_to_path(uri: &str) -> Option<PathBuf> {
    if let Some(path_str) = uri.strip_prefix("file://") {
        let decoded = urlencoding::decode(path_str).ok()?;
        Some(PathBuf::from(decoded.as_ref()))
    } else {
        None
    }
}

pub fn path_to_uri(path: &Path) -> String {
    let path_str = path.to_string_lossy();
    format!("file://{}", path_str)
}

pub fn load_imported_macros(
    file_path: &Path,
) -> Result<Vec<(String, Span, Vec<Statement>)>, String> {
    info!("Loading imported file: {:?}", file_path);

    let content = fs::read_to_string(file_path).map_err(|e| {
        let err = format!("Failed to read file {:?}: {}", file_path, e);
        info!("{}", err);
        err
    })?;

    info!("File loaded, {} bytes", content.len());

    let tokens = lexer::tokenize(&content).map_err(|e| format!("Tokenization failed: {:?}", e))?;

    let ast = parser::parse(tokens).map_err(|e| format!("Parsing failed: {:?}", e))?;

    let mut macros = Vec::new();
    for stmt in ast {
        if let Statement::MacroDef {
            name, span, body, ..
        } = stmt
        {
            info!("Found macro: {}", name);
            macros.push((name, span, body));
        }
    }

    info!("Loaded {} macros from {:?}", macros.len(), file_path);
    Ok(macros)
}
