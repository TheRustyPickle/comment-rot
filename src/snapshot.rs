use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::model::{Candidate, ScopedItem, Snapshot, SnapshotEntry};

const SNAPSHOT_FILENAME: &str = "snapshot.json";

pub struct DiffResult {
    /// Entries needing a human verdict: comment unchanged, code changed.
    pub candidates: Vec<Candidate>,
    pub new_baseline: Snapshot,
    pub added: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub pruned: usize,
}

impl Snapshot {
    fn path(root: &Path) -> PathBuf {
        root.join(".rot").join(SNAPSHOT_FILENAME)
    }

    #[must_use]
    pub fn exists(root: &Path) -> bool {
        Self::path(root).exists()
    }

    pub fn load(root: &Path) -> Result<Snapshot> {
        let path = Self::path(root);

        if !path.exists() {
            return Ok(Snapshot::new());
        }

        let data = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;

        serde_json::from_str(&data).with_context(|| format!("parsing {}", path.display()))
    }

    pub fn save(&self, root: &Path) -> Result<()> {
        let dir = root.join(".rot");
        std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        let path = Self::path(root);
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, data).with_context(|| format!("writing {}", path.display()))
    }

    /// Compares a fresh scan against this baseline. An entry becomes a
    /// candidate only when its comment hash is unchanged but its body hash
    /// isn't.
    #[must_use]
    pub fn diff(&self, fresh: &[ScopedItem]) -> DiffResult {
        let fresh_map: BTreeMap<&str, &ScopedItem> =
            fresh.iter().map(|i| (i.id.as_str(), i)).collect();

        let mut new_entries = BTreeMap::new();
        let mut candidates = Vec::new();
        let mut added = 0;
        let mut updated = 0;
        let mut unchanged = 0;

        for (id, item) in &fresh_map {
            let comment_hash = item.comment_hash();
            let body_hash = item.body_hash();

            match self.entries.get(*id) {
                None => {
                    added += 1;
                    new_entries.insert(id.to_string(), SnapshotEntry::from(*item));
                }
                Some(prev) => {
                    if prev.comment_hash == comment_hash && prev.body_hash != body_hash {
                        new_entries.insert(id.to_string(), prev.clone());

                        let to_push = Candidate {
                            id: id.to_string(),
                            kind: item.kind,
                            file: item.file.clone(),
                            item_path: item.item_path.clone(),
                            line: item.line,
                            comment_kind: item.comment_kind,
                            comment_text: item.comment_text.clone(),
                            old_body_text: prev.body_text.clone(),
                            new_body_text: item.body_text.clone(),
                            reason: format!(
                                "{} changed but its comment was not updated",
                                item.kind.label()
                            ),
                        };

                        candidates.push(to_push);
                    } else {
                        if prev.comment_hash != comment_hash || prev.body_hash != body_hash {
                            updated += 1;
                        } else {
                            unchanged += 1;
                        }

                        new_entries.insert(id.to_string(), SnapshotEntry::from(*item));
                    }
                }
            }
        }

        let pruned = self
            .entries
            .keys()
            .filter(|id| !fresh_map.contains_key(id.as_str()))
            .count();

        let mut snapshot = Snapshot::new();
        snapshot.entries = new_entries;

        DiffResult {
            candidates,
            new_baseline: snapshot,
            added,
            updated,
            unchanged,
            pruned,
        }
    }
}
