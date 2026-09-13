mod common;

use comment_rot::engine;
use comment_rot::model::{ConfirmInput, Verdict};
use common::Project;

#[test]
fn body_change_with_unchanged_comment_is_flagged() {
    let p = Project::new();
    p.write(
        "src/lib.rs",
        r#"
/// Adds two numbers.
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#,
    );
    engine::init(p.path(), false).unwrap();

    p.write(
        "src/lib.rs",
        r#"
/// Adds two numbers.
pub fn add(a: i32, b: i32) -> i32 {
    a + b + 1
}
"#,
    );
    let outcome = engine::check(p.path()).unwrap();
    assert_eq!(outcome.candidates.len(), 1);
    let c = &outcome.candidates[0];
    assert_eq!(c.item_path, "crate::add");
    assert!(c.old_body_text.contains("a + b"));
    assert!(c.new_body_text.contains("a + b + 1"));
}

#[test]
fn simultaneous_change_is_silently_absorbed_into_baseline() {
    let p = Project::new();
    p.write("src/lib.rs", "/// old doc\npub fn f() {}\n");
    engine::init(p.path(), false).unwrap();

    p.write(
        "src/lib.rs",
        r#"
/// new doc
pub fn f() {
    let _ = 1;
}
"#,
    );
    let outcome = engine::check(p.path()).unwrap();
    assert!(outcome.candidates.is_empty());
    assert_eq!(outcome.updated, 1);

    let outcome2 = engine::check(p.path()).unwrap();
    assert!(outcome2.candidates.is_empty());
    assert_eq!(outcome2.unchanged, 1);
}

#[test]
fn no_change_reports_unchanged_not_updated() {
    let p = Project::new();
    p.write("src/lib.rs", "/// doc\npub fn f() {}\n");
    engine::init(p.path(), false).unwrap();

    let outcome = engine::check(p.path()).unwrap();
    assert_eq!(outcome.unchanged, 1);
    assert_eq!(outcome.updated, 0);
    assert_eq!(outcome.added, 0);
    assert!(outcome.candidates.is_empty());
}

#[test]
fn renamed_item_is_pruned_and_readded_without_a_prompt() {
    let p = Project::new();
    p.write("src/lib.rs", "/// doc\npub fn old_name() {}\n");
    engine::init(p.path(), false).unwrap();

    p.write("src/lib.rs", "/// doc\npub fn new_name() {}\n");
    let outcome = engine::check(p.path()).unwrap();
    assert_eq!(outcome.pruned, 1);
    assert_eq!(outcome.added, 1);
    assert!(outcome.candidates.is_empty());
}

#[test]
fn confirming_yes_updates_baseline_so_it_stops_resurfacing() {
    let p = Project::new();
    p.write(
        "src/lib.rs",
        r#"
/// doc
pub fn f() -> i32 {
    1
}
"#,
    );
    engine::init(p.path(), false).unwrap();

    p.write(
        "src/lib.rs",
        r#"
/// doc
pub fn f() -> i32 {
    2
}
"#,
    );
    let outcome = engine::check(p.path()).unwrap();
    assert_eq!(outcome.candidates.len(), 1);
    let id = outcome.candidates[0].id.clone();

    let apply = engine::apply_verdicts(
        p.path(),
        &[ConfirmInput {
            id,
            verdict: Verdict::Yes,
        }],
    )
    .unwrap();
    assert_eq!(apply.updated, 1);
    assert_eq!(apply.flagged, 0);

    let outcome2 = engine::check(p.path()).unwrap();
    assert!(outcome2.candidates.is_empty());
    assert_eq!(outcome2.unchanged, 1);
}

#[test]
fn apply_verdicts_only_rescans_files_the_verdicts_touch() {
    let p = Project::new();
    p.write(
        "src/lib.rs",
        r#"
/// doc
pub fn f() -> i32 {
    1
}
"#,
    );
    engine::init(p.path(), false).unwrap();

    p.write(
        "src/lib.rs",
        r#"
/// doc
pub fn f() -> i32 {
    2
}
"#,
    );
    let outcome = engine::check(p.path()).unwrap();
    assert_eq!(outcome.candidates.len(), 1);
    let id = outcome.candidates[0].id.clone();

    std::fs::write(p.path().join("src/broken.rs"), [0xFF, 0xFE, 0x00, 0xFF]).unwrap();

    let apply = engine::apply_verdicts(
        p.path(),
        &[ConfirmInput {
            id,
            verdict: Verdict::Yes,
        }],
    )
    .unwrap();
    assert_eq!(apply.updated, 1);

    let outcome2 = engine::check(p.path());
    assert!(
        outcome2.is_err(),
        "sanity check: a full-project check must fail on the invalid-UTF-8 file"
    );
}

#[test]
fn apply_verdicts_skips_ids_that_no_longer_exist() {
    let p = Project::new();
    p.write("src/lib.rs", "/// doc\npub fn f() {}\n");
    engine::init(p.path(), false).unwrap();

    let apply = engine::apply_verdicts(
        p.path(),
        &[ConfirmInput {
            id: "crate::does_not_exist".to_string(),
            verdict: Verdict::No,
        }],
    )
    .unwrap();
    assert_eq!(apply.updated, 0);
    assert_eq!(apply.skipped_missing, 1);
}
