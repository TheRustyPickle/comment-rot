use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::hashing::hash_str;
use crate::model::{ItemKind, KnownIssue, KnownIssues, ScopedItem, now_unix};

const KNOWN_ISSUES_FILENAME: &str = "known_issues.json";

impl KnownIssues {
    fn path(root: &Path) -> PathBuf {
        root.join(".rot").join(KNOWN_ISSUES_FILENAME)
    }

    pub fn load(root: &Path) -> Result<KnownIssues> {
        let path = Self::path(root);

        if !path.exists() {
            return Ok(KnownIssues::new());
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

    /// Drops issues whose item no longer exists, and auto-clears issues
    /// whose comment text has since been edited
    pub fn reconcile(&mut self, fresh: &BTreeMap<String, &ScopedItem>) -> usize {
        let before = self.issues.len();

        self.issues.retain(|id, issue| match fresh.get(id) {
            None => false,
            Some(item) => hash_str(&item.comment_text) == hash_str(&issue.comment_text),
        });

        before - self.issues.len()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn flag(
        &mut self,
        id: &str,
        kind: ItemKind,
        file: &str,
        item_path: &str,
        comment_text: &str,
        old_body_text: &str,
        new_body_text: &str,
        reason: &str,
    ) {
        self.issues.insert(
            id.to_string(),
            KnownIssue {
                kind,
                file: file.to_string(),
                item_path: item_path.to_string(),
                flagged_at: now_unix(),
                comment_text: comment_text.to_string(),
                old_body_text: old_body_text.to_string(),
                new_body_text: new_body_text.to_string(),
                reason: reason.to_string(),
            },
        );
    }

    pub fn clear(&mut self, id: &str) -> bool {
        self.issues.remove(id).is_some()
    }
}
