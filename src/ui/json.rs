use anyhow::Result;
use rot::engine;
use rot::model::ConfirmInput;
use std::path::Path;

pub fn run_check(root: &Path) -> Result<()> {
    let outcome = engine::check(root)?;

    let payload = serde_json::json!({
        "candidates": outcome.candidates,
        "added": outcome.added,
        "updated": outcome.updated,
        "unchanged": outcome.unchanged,
        "pruned": outcome.pruned,
        "issues_cleared": outcome.issues_cleared,
    });

    println!("{}", serde_json::to_string_pretty(&payload)?);

    Ok(())
}

pub fn run_confirm(root: &Path) -> Result<()> {
    let verdicts: Vec<ConfirmInput> = serde_json::from_reader(std::io::stdin())?;
    let outcome = engine::apply_verdicts(root, &verdicts)?;

    let payload = serde_json::json!({
        "updated": outcome.updated,
        "flagged": outcome.flagged,
        "skipped_missing": outcome.skipped_missing,
    });

    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

pub fn run_status(root: &Path) -> Result<()> {
    let issues = engine::status(root)?;

    let payload: Vec<_> = issues
        .into_iter()
        .map(|(id, issue)| serde_json::json!({ "id": id, "issue": issue }))
        .collect();

    println!("{}", serde_json::to_string_pretty(&payload)?);

    Ok(())
}
