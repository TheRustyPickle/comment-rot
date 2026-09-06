mod common;

use common::Project;
use comment_rot::engine;

#[test]
fn init_creates_baseline_with_expected_entry_count() {
    let p = Project::new();
    p.write(
        "src/lib.rs",
        "/// Adds two numbers.\npub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n",
    );
    let outcome = engine::init(p.path(), false).unwrap();
    assert_eq!(outcome.entries, 1);
}

#[test]
fn init_ignores_items_with_no_comment() {
    let p = Project::new();
    p.write("src/lib.rs", "pub fn f() {}\npub struct S { pub a: i32 }\n");
    let outcome = engine::init(p.path(), false).unwrap();
    assert_eq!(outcome.entries, 0);
}

#[test]
fn init_refuses_to_overwrite_without_force() {
    let p = Project::new();
    p.write("src/lib.rs", "pub fn f() {}\n");
    engine::init(p.path(), false).unwrap();

    let err = engine::init(p.path(), false).unwrap_err();
    assert!(err.to_string().contains("already exists"));
}

#[test]
fn init_force_rebaselines_from_scratch() {
    let p = Project::new();
    p.write("src/lib.rs", "/// doc\npub fn f() {}\n");
    engine::init(p.path(), false).unwrap();

    p.write(
        "src/lib.rs",
        "/// doc\npub fn f() {}\n/// doc2\npub fn g() {}\n",
    );
    let outcome = engine::init(p.path(), true).unwrap();
    assert_eq!(outcome.entries, 2);
}

#[test]
fn check_without_init_errors() {
    let p = Project::new();
    p.write("src/lib.rs", "pub fn f() {}\n");
    let err = engine::check(p.path()).unwrap_err();
    assert!(err.to_string().contains("rot init"));
}
