use anyhow::Result;
use comment_rot::engine;
use comment_rot::model::ConfirmInput;
use comment_rot::model::Verdict;
use console::style;
use dialoguer::Confirm;
use similar::{ChangeTag, TextDiff};
use std::path::Path;

pub fn run_init(root: &Path, force: bool) -> Result<()> {
    let outcome = engine::init(root, force)?;

    println!(
        "{}",
        style(format!("snapshot created with {} entries", outcome.entries)).green()
    );

    Ok(())
}

pub fn run_check(root: &Path) -> Result<()> {
    let outcome = engine::check(root)?;

    println!(
        "{}",
        style(format!(
            "{} new, {} updated, {} unchanged, {} pruned, {} known issue(s) auto-cleared",
            outcome.added,
            outcome.updated,
            outcome.unchanged,
            outcome.pruned,
            outcome.issues_cleared
        ))
        .dim()
    );

    if outcome.candidates.is_empty() {
        println!("{}", style("No stale-comment candidates found.").green());
        return Ok(());
    }

    let total = outcome.candidates.len();
    let mut verdicts = Vec::with_capacity(total);

    for (i, c) in outcome.candidates.iter().enumerate() {
        println!();
        println!(
            "{}",
            style(format!(
                "[{}/{}] {} `{}` - {}:{}",
                i + 1,
                total,
                c.kind.label(),
                c.item_path,
                c.file,
                c.line
            ))
            .bold()
        );
        println!("{}", style("comment:").underlined());
        println!("{}", c.comment_text);
        println!("{}", style("what changed:").underlined());
        print_diff(&c.old_body_text, &c.new_body_text);
        println!("{}", style(&c.reason).yellow());

        let accurate = Confirm::new()
            .with_prompt("Is this comment still accurate?")
            .default(false)
            .interact()?;

        verdicts.push(ConfirmInput {
            id: c.id.clone(),
            verdict: if accurate { Verdict::Yes } else { Verdict::No },
        });
    }

    let apply = engine::apply_verdicts(root, &verdicts)?;

    println!();
    println!(
        "{} baseline entries updated, {} flagged as known issue(s).",
        apply.updated, apply.flagged
    );
    Ok(())
}

pub fn run_status(root: &Path) -> Result<()> {
    let issues = engine::status(root)?;

    if issues.is_empty() {
        println!("{}", style("No known issues.").green());
        return Ok(());
    }

    for (id, issue) in issues {
        println!(
            "{}",
            style(format!(
                "{} `{}` - {}",
                issue.kind.label(),
                issue.item_path,
                issue.file
            ))
            .bold()
        );
        println!("  id: {id}");
        println!("  reason: {}", issue.reason);
        if let Some(first_line) = issue.comment_text.lines().next() {
            println!("  comment: {first_line}");
        }
    }

    Ok(())
}

pub fn run_resolve(root: &Path, id: &str) -> Result<bool> {
    let resolved = engine::resolve(root, id)?;

    if resolved {
        println!("{}", style(format!("resolved {id}")).green());
    } else {
        println!("{}", style(format!("no known issue with id {id}")).yellow());
    }

    Ok(resolved)
}

fn print_diff(old: &str, new: &str) {
    let diff = TextDiff::from_lines(old, new);

    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };

        let line = format!("{sign}{change}");

        match change.tag() {
            ChangeTag::Delete => print!("{}", style(line).red()),
            ChangeTag::Insert => print!("{}", style(line).green()),
            ChangeTag::Equal => print!("{}", style(line).dim()),
        }
    }

    println!();
}
