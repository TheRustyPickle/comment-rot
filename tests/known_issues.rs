mod common;

use common::Project;
use comment_rot::engine;
use comment_rot::model::{Candidate, ConfirmInput, Verdict};

fn one_candidate(p: &Project) -> Candidate {
    let outcome = engine::check(p.path()).unwrap();
    assert_eq!(outcome.candidates.len(), 1);
    outcome.candidates.into_iter().next().unwrap()
}

#[test]
fn no_verdict_flags_a_known_issue_visible_via_status() {
    let p = Project::new();
    p.write("src/lib.rs", "/// doc\npub fn f() -> i32 {\n    1\n}\n");
    engine::init(p.path(), false).unwrap();
    p.write("src/lib.rs", "/// doc\npub fn f() -> i32 {\n    2\n}\n");
    let c = one_candidate(&p);

    let apply = engine::apply_verdicts(
        p.path(),
        &[ConfirmInput {
            id: c.id.clone(),
            verdict: Verdict::No,
        }],
    )
    .unwrap();
    assert_eq!(apply.flagged, 1);

    let issues = engine::status(p.path()).unwrap();
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].0, c.id);

    // Baseline moved forward, so it doesn't resurface as a fresh candidate.
    let outcome2 = engine::check(p.path()).unwrap();
    assert!(outcome2.candidates.is_empty());
}

#[test]
fn editing_the_comment_auto_clears_the_known_issue() {
    let p = Project::new();
    p.write("src/lib.rs", "/// old doc\npub fn f() -> i32 {\n    1\n}\n");
    engine::init(p.path(), false).unwrap();
    p.write("src/lib.rs", "/// old doc\npub fn f() -> i32 {\n    2\n}\n");
    let c = one_candidate(&p);
    engine::apply_verdicts(
        p.path(),
        &[ConfirmInput {
            id: c.id.clone(),
            verdict: Verdict::No,
        }],
    )
    .unwrap();
    assert_eq!(engine::status(p.path()).unwrap().len(), 1);

    p.write("src/lib.rs", "/// new doc\npub fn f() -> i32 {\n    2\n}\n");
    let outcome = engine::check(p.path()).unwrap();
    assert_eq!(outcome.issues_cleared, 1);
    assert!(engine::status(p.path()).unwrap().is_empty());
}

#[test]
fn resolve_manually_clears_an_issue() {
    let p = Project::new();
    p.write("src/lib.rs", "/// doc\npub fn f() -> i32 {\n    1\n}\n");
    engine::init(p.path(), false).unwrap();
    p.write("src/lib.rs", "/// doc\npub fn f() -> i32 {\n    2\n}\n");
    let c = one_candidate(&p);
    engine::apply_verdicts(
        p.path(),
        &[ConfirmInput {
            id: c.id.clone(),
            verdict: Verdict::No,
        }],
    )
    .unwrap();

    assert!(engine::resolve(p.path(), &c.id).unwrap());
    assert!(engine::status(p.path()).unwrap().is_empty());
    // Already gone - resolving again reports nothing found.
    assert!(!engine::resolve(p.path(), &c.id).unwrap());
}

#[test]
fn removing_the_item_prunes_its_known_issue() {
    let p = Project::new();
    p.write(
        "src/lib.rs",
        "/// doc\npub fn f() -> i32 {\n    1\n}\npub fn keep() {}\n",
    );
    engine::init(p.path(), false).unwrap();
    p.write(
        "src/lib.rs",
        "/// doc\npub fn f() -> i32 {\n    2\n}\npub fn keep() {}\n",
    );
    let c = one_candidate(&p);
    engine::apply_verdicts(
        p.path(),
        &[ConfirmInput {
            id: c.id.clone(),
            verdict: Verdict::No,
        }],
    )
    .unwrap();
    assert_eq!(engine::status(p.path()).unwrap().len(), 1);

    p.write("src/lib.rs", "pub fn keep() {}\n");
    engine::check(p.path()).unwrap();
    assert!(engine::status(p.path()).unwrap().is_empty());
}

#[test]
fn yes_verdict_clears_a_previously_flagged_issue() {
    let p = Project::new();
    p.write("src/lib.rs", "/// doc\npub fn f() -> i32 {\n    1\n}\n");
    engine::init(p.path(), false).unwrap();
    p.write("src/lib.rs", "/// doc\npub fn f() -> i32 {\n    2\n}\n");
    let c = one_candidate(&p);
    engine::apply_verdicts(
        p.path(),
        &[ConfirmInput {
            id: c.id.clone(),
            verdict: Verdict::No,
        }],
    )
    .unwrap();
    assert_eq!(engine::status(p.path()).unwrap().len(), 1);

    // A later re-review (e.g. via a second `confirm` call) can say "yes"
    // and clear it without needing the comment text to change.
    engine::apply_verdicts(
        p.path(),
        &[ConfirmInput {
            id: c.id,
            verdict: Verdict::Yes,
        }],
    )
    .unwrap();
    assert!(engine::status(p.path()).unwrap().is_empty());
}
