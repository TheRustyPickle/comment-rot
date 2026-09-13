mod common;

use comment_rot::engine;
use comment_rot::model::{ConfirmInput, Verdict};
use common::Project;

#[test]
fn same_path_in_two_files_does_not_collide() {
    let p = Project::new();
    p.write("src/lib.rs", "/// doc for lib\npub fn helper() {}\n");
    p.write("src/bin/tool.rs", "/// doc for tool\nfn helper() {}\n");

    let outcome = engine::init(p.path(), false).unwrap();
    assert_eq!(
        outcome.entries, 2,
        "each file's `helper` must be tracked separately"
    );

    p.write(
        "src/lib.rs",
        r#"
/// doc for lib
pub fn helper() {
    1
}
"#,
    );
    let check = engine::check(p.path()).unwrap();
    assert_eq!(check.candidates.len(), 1);
    assert_eq!(check.candidates[0].file, "src/lib.rs");
}

#[test]
fn free_comment_id_stays_stable_while_its_code_changes() {
    let p = Project::new();
    p.write(
        "src/lib.rs",
        r#"
pub fn f() {
    // note
    let x = 1;
}
"#,
    );
    engine::init(p.path(), false).unwrap();

    p.write(
        "src/lib.rs",
        r#"
pub fn f() {
    // note
    let x = 999;
}
"#,
    );
    let outcome = engine::check(p.path()).unwrap();
    assert_eq!(
        outcome.added, 0,
        "the id must not change just because the code did"
    );
    assert_eq!(outcome.pruned, 0);
    assert_eq!(outcome.candidates.len(), 1);
}

#[test]
fn duplicate_identical_comments_in_one_function_are_tracked_independently() {
    let p = Project::new();
    p.write(
        "src/lib.rs",
        r#"
pub fn f() {
    // note
    let a = 1;
    // note
    let b = 2;
}
"#,
    );
    let outcome = engine::init(p.path(), false).unwrap();
    assert_eq!(
        outcome.entries, 2,
        "each occurrence needs its own id, not one shared id"
    );

    p.write(
        "src/lib.rs",
        r#"
pub fn f() {
    // note
    let a = 100;
    // note
    let b = 2;
}
"#,
    );
    let check = engine::check(p.path()).unwrap();
    assert_eq!(check.candidates.len(), 1);
    assert!(check.candidates[0].old_body_text.contains("let a = 1"));
}

#[test]
fn free_comment_end_to_end_confirm_roundtrip() {
    let p = Project::new();
    p.write(
        "src/lib.rs",
        r#"
pub fn f() {
    // uses the default limit
    let limit = 10;
}
"#,
    );
    engine::init(p.path(), false).unwrap();

    p.write(
        "src/lib.rs",
        r#"
pub fn f() {
    // uses the default limit
    let limit = 20;
}
"#,
    );
    let outcome = engine::check(p.path()).unwrap();
    assert_eq!(outcome.candidates.len(), 1);
    let id = outcome.candidates[0].id.clone();

    engine::apply_verdicts(
        p.path(),
        &[ConfirmInput {
            id,
            verdict: Verdict::No,
        }],
    )
    .unwrap();
    assert_eq!(engine::status(p.path()).unwrap().len(), 1);
}
