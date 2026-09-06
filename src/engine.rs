use anyhow::{Result, bail};
use std::collections::BTreeMap;
use std::path::Path;

use crate::model::{
    Candidate, ConfirmInput, KnownIssue, KnownIssues, ScopedItem, Snapshot, SnapshotEntry, Verdict,
};
use crate::scan;

#[derive(Debug)]
pub struct InitOutcome {
    pub entries: usize,
}

pub fn init(root: &Path, force: bool) -> Result<InitOutcome> {
    if Snapshot::exists(root) && !force {
        bail!(".rot/snapshot.json already exists - pass --force to re-baseline from scratch");
    }

    let fresh = scan::scan_project(root)?;
    let mut snap = Snapshot::new();

    for item in &fresh {
        snap.entries
            .insert(item.id.clone(), SnapshotEntry::from(item));
    }

    let entries = snap.entries.len();

    snap.save(root)?;
    KnownIssues::new().save(root)?;

    Ok(InitOutcome { entries })
}

#[derive(Debug)]
pub struct CheckOutcome {
    pub candidates: Vec<Candidate>,
    pub added: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub pruned: usize,
    pub issues_cleared: usize,
}

/// Scans the project, diffs against the stored baseline, and
/// persists everything that doesn't need a manual verdict.
/// Only real candidates are held back
pub fn check(root: &Path) -> Result<CheckOutcome> {
    if !Snapshot::exists(root) {
        bail!("no .rot/snapshot.json found - run `rot init` first");
    }

    let old = Snapshot::load(root)?;
    let fresh = scan::scan_project(root)?;

    let diff = old.diff(&fresh);

    diff.new_baseline.save(root)?;

    let fresh_map = fresh.iter().map(|i| (i.id.clone(), i)).collect();

    let mut issues = KnownIssues::load(root)?;

    let issues_cleared = issues.reconcile(&fresh_map);

    issues.save(root)?;

    Ok(CheckOutcome {
        candidates: diff.candidates,
        added: diff.added,
        updated: diff.updated,
        unchanged: diff.unchanged,
        pruned: diff.pruned,
        issues_cleared,
    })
}

#[derive(Debug)]
pub struct ApplyOutcome {
    pub updated: usize,
    pub flagged: usize,
    pub skipped_missing: usize,
}

/// Applies verdicts to the current baseline.
/// Re-scans the relevant files before applying the verdicts
pub fn apply_verdicts(root: &Path, verdicts: &[ConfirmInput]) -> Result<ApplyOutcome> {
    let mut snap = Snapshot::load(root)?;

    let fresh = scan::scan_files_for_ids(root, verdicts.iter().map(|v| v.id.as_str()))?;
    let fresh_map: BTreeMap<String, &ScopedItem> =
        fresh.iter().map(|i| (i.id.clone(), i)).collect();

    let mut issues = KnownIssues::load(root)?;

    let mut updated = 0;
    let mut flagged = 0;
    let mut skipped_missing = 0;

    for v in verdicts {
        let Some(fresh_item) = fresh_map.get(&v.id) else {
            skipped_missing += 1;
            continue;
        };

        let old_entry = snap.entries.get(v.id.as_str()).cloned();

        // Update existing snap with the item from fresh scan
        snap.entries
            .insert(v.id.clone(), SnapshotEntry::from(*fresh_item));

        updated += 1;

        match v.verdict {
            Verdict::Yes => {
                issues.clear(&v.id);
            }
            Verdict::No => {
                let old_body = old_entry.map(|e| e.body_text).unwrap_or_default();

                issues.flag(
                    &v.id,
                    fresh_item.kind,
                    &fresh_item.file,
                    &fresh_item.item_path,
                    &fresh_item.comment_text,
                    &old_body,
                    &fresh_item.body_text,
                    "confirmed stale by reviewer",
                );

                flagged += 1;
            }
        }
    }

    snap.save(root)?;
    issues.save(root)?;

    Ok(ApplyOutcome {
        updated,
        flagged,
        skipped_missing,
    })
}

pub fn status(root: &Path) -> Result<Vec<(String, KnownIssue)>> {
    let issues = KnownIssues::load(root)?;
    Ok(issues.issues.into_iter().collect())
}

pub fn resolve(root: &Path, id: &str) -> Result<bool> {
    let mut issues = KnownIssues::load(root)?;
    let removed = issues.clear(id);

    issues.save(root)?;

    Ok(removed)
}
