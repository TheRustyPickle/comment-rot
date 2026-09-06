use anyhow::{Context, Result};
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

/// Walks `root` respecting `.gitignore` and returns every `.rs` file found
pub fn collect_rust_files(root: &Path) -> Result<Vec<String>> {
    let mut files = Vec::new();

    let walker = WalkBuilder::new(root)
        .hidden(true)
        .require_git(false)
        .build();

    for entry in walker {
        let entry = entry.context("walking project directory")?;
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }

        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }

        let rel = relative_slash_path(root, path)?;
        files.push(rel);
    }

    files.sort();
    Ok(files)
}

fn relative_slash_path(root: &Path, path: &Path) -> Result<String> {
    let rel = path
        .strip_prefix(root)
        .with_context(|| format!("{} is not under {}", path.display(), root.display()))?;

    let parts: Vec<&str> = rel
        .components()
        .map(|c| c.as_os_str().to_str().unwrap_or_default())
        .collect();

    Ok(parts.join("/"))
}

pub fn to_abs(root: &Path, rel: &str) -> PathBuf {
    root.join(rel)
}
