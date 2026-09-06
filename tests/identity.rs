mod common;

use common::Project;
use rot::engine;
use rot::model::{ConfirmInput, Verdict};

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

    // Changing only one file's `helper` body must flag only that one.
    p.write(
        "src/lib.rs",
        "/// doc for lib\npub fn helper() {\n    1\n}\n",
    );
    let check = engine::check(p.path()).unwrap();
    assert_eq!(check.candidates.len(), 1);
    assert_eq!(check.candidates[0].file, "src/lib.rs");
}

#[test]
fn free_comment_id_stays_stable_while_its_code_changes() {
    // This is the actual staleness-detection contract for regular
    // comments: the id must NOT be derived from the code being described,
    // or a code change would always look like "old entry gone, new entry
    // added" instead of "same entry, body drifted".
    let p = Project::new();
    p.write(
        "src/lib.rs",
        "pub fn f() {\n    // note\n    let x = 1;\n}\n",
    );
    engine::init(p.path(), false).unwrap();

    p.write(
        "src/lib.rs",
        "pub fn f() {\n    // note\n    let x = 999;\n}\n",
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
        "pub fn f() {\n    // note\n    let a = 1;\n    // note\n    let b = 2;\n}\n",
    );
    let outcome = engine::init(p.path(), false).unwrap();
    assert_eq!(
        outcome.entries, 2,
        "each occurrence needs its own id, not one shared id"
    );

    // Only the first one's code changes - only it should be flagged.
    p.write(
        "src/lib.rs",
        "pub fn f() {\n    // note\n    let a = 100;\n    // note\n    let b = 2;\n}\n",
    );
    let check = engine::check(p.path()).unwrap();
    assert_eq!(check.candidates.len(), 1);
    assert!(check.candidates[0].old_body_text.contains("let a = 1"));
}

#[test]
fn free_comment_end_to_end_confirm_roundtrip() {
    // Same round trip as the doc-comment tests, but for a plain `//`
    // comment - this is the case the two bugs above were masking.
    let p = Project::new();
    p.write(
        "src/lib.rs",
        "pub fn f() {\n    // uses the default limit\n    let limit = 10;\n}\n",
    );
    engine::init(p.path(), false).unwrap();

    p.write(
        "src/lib.rs",
        "pub fn f() {\n    // uses the default limit\n    let limit = 20;\n}\n",
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
