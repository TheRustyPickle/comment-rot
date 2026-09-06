mod common;

use comment_rot::engine;
use common::Project;

#[test]
fn ignored_files_are_never_scanned_even_without_a_git_repo() {
    let p = Project::new();
    p.write(".gitignore", "/target\n");
    p.write("src/lib.rs", "/// doc\npub fn f() {}\n");
    p.write(
        "target/debug/build/generated.rs",
        "/// generated\npub fn g() {}\n/// another\npub fn h() {}\n",
    );

    let outcome = engine::init(p.path(), false).unwrap();
    assert_eq!(
        outcome.entries, 1,
        "only src/lib.rs should be scanned, target/ is gitignored"
    );
}

#[test]
fn adding_a_file_under_an_ignored_dir_never_shows_up_as_added() {
    let p = Project::new();
    p.write(".gitignore", "/target\n");
    p.write("src/lib.rs", "/// doc\npub fn f() {}\n");
    engine::init(p.path(), false).unwrap();

    p.write("target/debug/whatever.rs", "/// x\npub fn y() {}\n");
    let outcome = engine::check(p.path()).unwrap();
    assert_eq!(outcome.added, 0);
    assert_eq!(outcome.unchanged, 1);
}

#[test]
fn nested_gitignore_is_respected() {
    let p = Project::new();
    p.write("src/lib.rs", "/// doc\npub fn f() {}\n");
    p.write("vendor/.gitignore", "*\n");
    p.write("vendor/thirdparty.rs", "/// ignored\npub fn z() {}\n");

    let outcome = engine::init(p.path(), false).unwrap();
    assert_eq!(outcome.entries, 1);
}
