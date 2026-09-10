mod common;

use comment_rot::engine;
use comment_rot::model::{Candidate, ItemKind};
use common::Project;

fn candidate_for(p: &Project, before: &str, after: &str) -> Candidate {
    p.write("src/lib.rs", before);
    engine::init(p.path(), false).unwrap();
    p.write("src/lib.rs", after);
    let outcome = engine::check(p.path()).unwrap();
    assert_eq!(
        outcome.candidates.len(),
        1,
        "expected exactly one candidate, got {:#?}",
        outcome.candidates
    );
    outcome.candidates.into_iter().next().unwrap()
}

#[test]
fn function_scope_hashes_the_full_body() {
    let p = Project::new();
    let c = candidate_for(
        &p,
        "/// doc\npub fn f(a: i32) -> i32 {\n    a\n}\n",
        "/// doc\npub fn f(a: i32) -> i32 {\n    a + 1\n}\n",
    );
    assert_eq!(c.kind, ItemKind::Function);
    assert_eq!(c.item_path, "crate::f");
}

#[test]
fn struct_scope_tracks_its_field_list() {
    let p = Project::new();
    let c = candidate_for(
        &p,
        "/// doc\npub struct S {\n    pub a: i32,\n}\n",
        "/// doc\npub struct S {\n    pub a: i32,\n    pub b: i32,\n}\n",
    );
    assert_eq!(c.kind, ItemKind::Struct);
    assert_eq!(c.item_path, "crate::S");
}

#[test]
fn field_scope_is_independent_of_the_struct() {
    let p = Project::new();
    let c = candidate_for(
        &p,
        "pub struct S {\n    /// doc\n    pub a: i32,\n}\n",
        "pub struct S {\n    /// doc\n    pub a: u64,\n}\n",
    );
    assert_eq!(c.kind, ItemKind::Field);
    assert_eq!(c.item_path, "crate::S::a");
}

#[test]
fn enum_variant_scope() {
    let p = Project::new();
    let c = candidate_for(
        &p,
        "pub enum E {\n    /// doc\n    A(i32),\n}\n",
        "pub enum E {\n    /// doc\n    A(u64),\n}\n",
    );
    assert_eq!(c.kind, ItemKind::Variant);
    assert_eq!(c.item_path, "crate::E::A");
}

#[test]
fn field_inside_a_struct_like_enum_variant_is_its_own_scope() {
    // Regression: a doc comment on a field inside a *struct-like* enum
    // variant (`Variant { field: T }`) used to be invisible entirely - the
    // recursion into nested containers didn't descend into `enum_variant`
    // bodies, so the doc comment was silently stripped as a nested comment
    // instead of being tracked.
    let p = Project::new();
    let c = candidate_for(
        &p,
        "pub enum E {\n    Check {\n        path: i32,\n        /// doc\n        json: bool,\n    },\n}\n",
        "pub enum E {\n    Check {\n        path: i32,\n        /// doc\n        json: u8,\n    },\n}\n",
    );
    assert_eq!(c.kind, ItemKind::Field);
    assert_eq!(c.item_path, "crate::E::Check::json");
}

#[test]
fn doc_comment_attaches_through_an_intervening_attribute() {
    // Regression: `#[arg(long)]` (or any attribute) between a doc comment
    // and the item it documents is a sibling node in the parse tree, not
    // part of the item - the attachment check used to look only at the
    // *very next* sibling, see it wasn't item-like, and misclassify the
    // doc comment as a free comment instead. Idiomatic clap-derive code
    // (doc comment, then `#[arg(...)]`, then the field) hits this directly.
    let p = Project::new();
    let c = candidate_for(
        &p,
        "pub struct S {\n    /// doc\n    #[arg(long)]\n    pub force: i32,\n}\n",
        "pub struct S {\n    /// doc\n    #[arg(long)]\n    pub force: u64,\n}\n",
    );
    assert_eq!(c.kind, ItemKind::Field);
    assert_eq!(c.item_path, "crate::S::force");
}

#[test]
fn macro_rules_scope() {
    // Regression: `macro_rules!` definitions weren't in the item allowlist,
    // so a doc comment on one fell through to free-comment scoping instead
    // of attaching to the macro.
    let p = Project::new();
    let c = candidate_for(
        &p,
        "/// doc\nmacro_rules! double {\n    ($x:expr) => {\n        $x * 2\n    };\n}\n",
        "/// doc\nmacro_rules! double {\n    ($x:expr) => {\n        $x * 3\n    };\n}\n",
    );
    assert_eq!(c.kind, ItemKind::Macro);
    assert_eq!(c.item_path, "crate::double");
}

#[test]
fn trait_associated_type_scope() {
    // Regression: a trait's associated type declaration (`type Item;`)
    // parses as its own `associated_type` node, distinct from `type_item`
    // (a real type alias) - it wasn't in the item allowlist either, so its
    // doc comment fell through to free-comment scoping too.
    let p = Project::new();
    let c = candidate_for(
        &p,
        "pub trait T {\n    /// doc\n    type Item;\n}\n",
        "pub trait T {\n    /// doc\n    type Item: Clone;\n}\n",
    );
    assert_eq!(c.kind, ItemKind::TypeAlias);
    assert_eq!(c.item_path, "crate::T::Item");
}

#[test]
fn trait_scope_reacts_to_a_new_member_signature() {
    let p = Project::new();
    let c = candidate_for(
        &p,
        "/// doc\npub trait T {\n    fn a(&self);\n}\n",
        "/// doc\npub trait T {\n    fn a(&self);\n    fn b(&self);\n}\n",
    );
    assert_eq!(c.kind, ItemKind::Trait);
    assert_eq!(c.item_path, "crate::T");
}

#[test]
fn impl_scope_ignores_method_body_edits() {
    // The impl block's own doc is hashed against its *shallow* member
    // signature list, not full method bodies - so a body-only edit inside
    // a method must NOT flag the impl-level doc comment.
    let p = Project::new();
    p.write(
        "src/lib.rs",
        "pub struct S;\n/// doc\nimpl S {\n    pub fn a(&self) -> i32 {\n        1\n    }\n}\n",
    );
    engine::init(p.path(), false).unwrap();

    p.write(
        "src/lib.rs",
        "pub struct S;\n/// doc\nimpl S {\n    pub fn a(&self) -> i32 {\n        2\n    }\n}\n",
    );
    let outcome = engine::check(p.path()).unwrap();
    assert!(
        outcome.candidates.is_empty(),
        "impl doc should be shallow, got {:#?}",
        outcome.candidates
    );
}

#[test]
fn const_scope() {
    let p = Project::new();
    let c = candidate_for(
        &p,
        "/// doc\npub const N: i32 = 1;\n",
        "/// doc\npub const N: i32 = 2;\n",
    );
    assert_eq!(c.kind, ItemKind::Const);
}

#[test]
fn module_scope_is_shallow_over_item_signatures() {
    let p = Project::new();
    let c = candidate_for(
        &p,
        "//! module doc\npub fn f() {}\n",
        "//! module doc\npub fn f(a: i32) {}\n",
    );
    assert_eq!(c.kind, ItemKind::Module);
    assert_eq!(c.item_path, "crate");
}

#[test]
fn module_scope_ignores_unrelated_body_edits() {
    let p = Project::new();
    p.write(
        "src/lib.rs",
        "//! module doc\npub fn f() -> i32 {\n    1\n}\n",
    );
    engine::init(p.path(), false).unwrap();

    p.write(
        "src/lib.rs",
        "//! module doc\npub fn f() -> i32 {\n    2\n}\n",
    );
    let outcome = engine::check(p.path()).unwrap();
    assert!(
        outcome.candidates.is_empty(),
        "module doc should be shallow, got {:#?}",
        outcome.candidates
    );
}

#[test]
fn leading_free_comment_scopes_forward() {
    let p = Project::new();
    let c = candidate_for(
        &p,
        "pub fn f() {\n    // note\n    let x = 1;\n}\n",
        "pub fn f() {\n    // note\n    let x = 2;\n}\n",
    );
    assert_eq!(c.kind, ItemKind::Free);
    assert!(c.id.contains("#free:"), "id was {}", c.id);
    assert!(c.old_body_text.contains("let x = 1"));
}

#[test]
fn trailing_side_comment_scopes_backward() {
    let p = Project::new();
    let c = candidate_for(
        &p,
        "pub fn f() {\n    let x = 1; // note\n    let _ = x;\n}\n",
        "pub fn f() {\n    let x = 2; // note\n    let _ = x;\n}\n",
    );
    assert_eq!(c.kind, ItemKind::Free);
    assert!(c.id.contains("#trailing:"), "id was {}", c.id);
    assert!(c.old_body_text.contains("let x = 1"));
    assert!(!c.old_body_text.contains("let _"));
}

#[test]
fn reformatting_alone_never_creates_a_candidate() {
    // Hashing is whitespace-normalized, so pure reindentation/reformatting
    // must not be mistaken for a real body change.
    let p = Project::new();
    p.write(
        "src/lib.rs",
        "/// doc\npub fn f(a: i32, b: i32) -> i32 {\n    a + b\n}\n",
    );
    engine::init(p.path(), false).unwrap();

    p.write(
        "src/lib.rs",
        "/// doc\npub fn f(a: i32, b: i32) -> i32 {\n        a\n        +\n        b\n}\n",
    );
    let outcome = engine::check(p.path()).unwrap();
    assert!(outcome.candidates.is_empty());
    assert_eq!(outcome.unchanged, 1);
}
