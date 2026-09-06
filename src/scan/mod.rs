mod extract;
mod walk;

use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::path::Path;

use crate::model::ScopedItem;

/// Walks `root` respecting `.gitignore` and extracts every comment scope
/// from every `.rs` file found.
pub fn scan_project(root: &Path) -> Result<Vec<ScopedItem>> {
    let files = walk::collect_rust_files(root)?;

    let mut out = Vec::new();

    for rel in files {
        let abs = walk::to_abs(root, &rel);
        let src =
            std::fs::read_to_string(&abs).with_context(|| format!("reading {}", abs.display()))?;

        let items = extract::extract_file(&rel, &src)
            .with_context(|| format!("parsing {}", abs.display()))?;

        out.extend(items);
    }

    Ok(out)
}

/// Extracts comment scopes only from the files referenced by `ids`
pub fn scan_files_for_ids<'a>(
    root: &Path,
    ids: impl IntoIterator<Item = &'a str>,
) -> Result<Vec<ScopedItem>> {
    let files: BTreeSet<&str> = ids
        .into_iter()
        .filter_map(|id| id.split_once('|').map(|(file, _)| file))
        .collect();

    let mut out = Vec::new();

    for rel in files {
        let abs = walk::to_abs(root, rel);
        let Ok(src) = std::fs::read_to_string(&abs) else {
            continue; // file gone/renamed since the id was issued
        };

        let items = extract::extract_file(rel, &src)
            .with_context(|| format!("parsing {}", abs.display()))?;

        out.extend(items);
    }

    Ok(out)
}
