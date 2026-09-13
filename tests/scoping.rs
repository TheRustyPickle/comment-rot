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
        r#"
/// doc
pub fn f(a: i32) -> i32 {
    a
}
"#,
        r#"
/// doc
pub fn f(a: i32) -> i32 {
    a + 1
}
"#,
    );
    assert_eq!(c.kind, ItemKind::Function);
    assert_eq!(c.item_path, "crate::f");
}

#[test]
fn struct_scope_tracks_its_field_list() {
    let p = Project::new();
    let c = candidate_for(
        &p,
        r#"
/// doc
pub struct S {
    pub a: i32,
}
"#,
        r#"
/// doc
pub struct S {
    pub a: i32,
    pub b: i32,
}
"#,
    );
    assert_eq!(c.kind, ItemKind::Struct);
    assert_eq!(c.item_path, "crate::S");
}

#[test]
fn field_scope_is_independent_of_the_struct() {
    let p = Project::new();
    let c = candidate_for(
        &p,
        r#"
pub struct S {
    /// doc
    pub a: i32,
}
"#,
        r#"
pub struct S {
    /// doc
    pub a: u64,
}
"#,
    );
    assert_eq!(c.kind, ItemKind::Field);
    assert_eq!(c.item_path, "crate::S::a");
}

#[test]
fn enum_variant_scope() {
    let p = Project::new();
    let c = candidate_for(
        &p,
        r#"
pub enum E {
    /// doc
    A(i32),
}
"#,
        r#"
pub enum E {
    /// doc
    A(u64),
}
"#,
    );
    assert_eq!(c.kind, ItemKind::Variant);
    assert_eq!(c.item_path, "crate::E::A");
}

#[test]
fn field_inside_a_struct_like_enum_variant_is_its_own_scope() {
    let p = Project::new();
    let c = candidate_for(
        &p,
        r#"
pub enum E {
    Check {
        path: i32,
        /// doc
        json: bool,
    },
}
"#,
        r#"
pub enum E {
    Check {
        path: i32,
        /// doc
        json: u8,
    },
}
"#,
    );
    assert_eq!(c.kind, ItemKind::Field);
    assert_eq!(c.item_path, "crate::E::Check::json");
}

#[test]
fn doc_comment_attaches_through_an_intervening_attribute() {
    let p = Project::new();
    let c = candidate_for(
        &p,
        r#"
pub struct S {
    /// doc
    #[arg(long)]
    pub force: i32,
}
"#,
        r#"
pub struct S {
    /// doc
    #[arg(long)]
    pub force: u64,
}
"#,
    );
    assert_eq!(c.kind, ItemKind::Field);
    assert_eq!(c.item_path, "crate::S::force");
}

#[test]
fn macro_rules_scope() {
    let p = Project::new();
    let c = candidate_for(
        &p,
        r#"
/// doc
macro_rules! double {
    ($x:expr) => {
        $x * 2
    };
}
"#,
        r#"
/// doc
macro_rules! double {
    ($x:expr) => {
        $x * 3
    };
}
"#,
    );
    assert_eq!(c.kind, ItemKind::Macro);
    assert_eq!(c.item_path, "crate::double");
}

#[test]
fn trait_associated_type_scope() {
    let p = Project::new();
    let c = candidate_for(
        &p,
        r#"
pub trait T {
    /// doc
    type Item;
}
"#,
        r#"
pub trait T {
    /// doc
    type Item: Clone;
}
"#,
    );
    assert_eq!(c.kind, ItemKind::TypeAlias);
    assert_eq!(c.item_path, "crate::T::Item");
}

#[test]
fn extern_block_scope() {
    let p = Project::new();
    let c = candidate_for(
        &p,
        r#"
/// doc
extern "C" {
    fn foo();
}
"#,
        r#"
/// doc
extern "C" {
    fn foo(x: i32);
}
"#,
    );
    assert_eq!(c.kind, ItemKind::Extern);
    assert_eq!(c.item_path, "crate::extern \"C\"");
}

#[test]
fn extern_block_body_is_recursed_into() {
    let p = Project::new();
    let c = candidate_for(
        &p,
        r#"
extern "C" {
    /// doc
    fn foo(x: i32);
}
"#,
        r#"
extern "C" {
    /// doc
    fn foo(x: i64);
}
"#,
    );
    assert_eq!(c.kind, ItemKind::Function);
    assert_eq!(c.item_path, "crate::extern \"C\"::foo");
}

#[test]
fn attribute_only_change_is_visible_to_the_field_it_decorates() {
    let p = Project::new();
    let c = candidate_for(
        &p,
        r#"
pub struct S {
    /// doc
    #[arg(long, short)]
    pub force: bool,
}
"#,
        r#"
pub struct S {
    /// doc
    #[arg(long)]
    pub force: bool,
}
"#,
    );
    assert_eq!(c.kind, ItemKind::Field);
    assert_eq!(c.item_path, "crate::S::force");
}

#[test]
fn attribute_only_change_surfaces_at_the_field_not_just_the_enclosing_variant() {
    let p = Project::new();
    p.write(
        "src/lib.rs",
        r#"
pub enum Command {
    /// Manually clear a known-issue entry by id
    Resolve {
        id: String,
        /// Print the result as JSON instead of plain text
        #[arg(long, short)]
        json: bool,
    },
}
"#,
    );
    engine::init(p.path(), false).unwrap();

    p.write(
        "src/lib.rs",
        r#"
pub enum Command {
    /// Manually clear a known-issue entry by id
    Resolve {
        id: String,
        /// Print the result as JSON instead of plain text
        #[arg(long)]
        json: bool,
    },
}
"#,
    );
    let outcome = engine::check(p.path()).unwrap();
    assert_eq!(
        outcome.candidates.len(),
        2,
        "expected both the variant and the field to surface, got {:#?}",
        outcome.candidates
    );

    let variant = outcome
        .candidates
        .iter()
        .find(|c| c.item_path == "crate::Command::Resolve")
        .expect("variant-level candidate");
    assert_eq!(variant.kind, ItemKind::Variant);

    let field = outcome
        .candidates
        .iter()
        .find(|c| c.item_path == "crate::Command::Resolve::json")
        .expect("field-level candidate");
    assert_eq!(field.kind, ItemKind::Field);
    assert_eq!(field.line, 6);
}

#[test]
fn comment_inside_a_closure_body_is_its_own_scope() {
    let p = Project::new();
    let c = candidate_for(
        &p,
        r#"
pub fn f(v: Vec<i32>) -> Vec<i32> {
    v.into_iter()
        .map(|x| {
            // double it
            x * 2
        })
        .collect()
}
"#,
        r#"
pub fn f(v: Vec<i32>) -> Vec<i32> {
    v.into_iter()
        .map(|x| {
            // double it
            x * 3
        })
        .collect()
}
"#,
    );
    assert_eq!(c.kind, ItemKind::Free);
    assert!(c.comment_text.contains("double it"));
}

#[test]
fn free_comment_before_a_closure_call_does_not_react_to_changes_inside_it() {
    let p = Project::new();
    p.write(
        "src/lib.rs",
        r#"
pub fn f(v: Vec<i32>) -> Vec<i32> {
    // process items
    v.into_iter()
        .map(|x| {
            x * 2
        })
        .collect()
}
"#,
    );
    engine::init(p.path(), false).unwrap();

    p.write(
        "src/lib.rs",
        r#"
pub fn f(v: Vec<i32>) -> Vec<i32> {
    // process items
    v.into_iter()
        .map(|x| {
            x * 3
        })
        .collect()
}
"#,
    );
    let outcome = engine::check(p.path()).unwrap();
    assert!(
        outcome.candidates.is_empty(),
        "the closure body has no comment of its own, got {:#?}",
        outcome.candidates
    );
}

#[test]
fn comment_inside_an_unsafe_block_is_its_own_scope() {
    let p = Project::new();
    let c = candidate_for(
        &p,
        r#"
pub fn f(a: i32) -> i32 {
    unsafe {
        // unsafe note
        a + 1
    }
}
"#,
        r#"
pub fn f(a: i32) -> i32 {
    unsafe {
        // unsafe note
        a + 2
    }
}
"#,
    );
    assert_eq!(c.kind, ItemKind::Free);
    assert!(c.comment_text.contains("unsafe note"));
}

#[test]
fn comment_inside_a_bare_scoping_block_is_its_own_scope() {
    let p = Project::new();
    let c = candidate_for(
        &p,
        r#"
pub fn f(a: i32) -> i32 {
    {
        // bare block note
        a + 1
    }
}
"#,
        r#"
pub fn f(a: i32) -> i32 {
    {
        // bare block note
        a + 2
    }
}
"#,
    );
    assert_eq!(c.kind, ItemKind::Free);
    assert!(c.comment_text.contains("bare block note"));
}

#[test]
fn comment_inside_control_flow_used_as_a_let_value_is_its_own_scope() {
    let p = Project::new();
    let c = candidate_for(
        &p,
        r#"
pub fn f(a: i32) -> i32 {
    let x = match a {
        0 => 0,
        _ => {
            // match-as-value note
            a + 1
        }
    };
    x
}
"#,
        r#"
pub fn f(a: i32) -> i32 {
    let x = match a {
        0 => 0,
        _ => {
            // match-as-value note
            a + 2
        }
    };
    x
}
"#,
    );
    assert_eq!(c.kind, ItemKind::Free);
    assert!(c.comment_text.contains("match-as-value note"));
}

#[test]
fn trait_scope_reacts_to_a_new_member_signature() {
    let p = Project::new();
    let c = candidate_for(
        &p,
        r#"
/// doc
pub trait T {
    fn a(&self);
}
"#,
        r#"
/// doc
pub trait T {
    fn a(&self);
    fn b(&self);
}
"#,
    );
    assert_eq!(c.kind, ItemKind::Trait);
    assert_eq!(c.item_path, "crate::T");
}

#[test]
fn impl_scope_ignores_method_body_edits() {
    let p = Project::new();
    p.write(
        "src/lib.rs",
        r#"
pub struct S;
/// doc
impl S {
    pub fn a(&self) -> i32 {
        1
    }
}
"#,
    );
    engine::init(p.path(), false).unwrap();

    p.write(
        "src/lib.rs",
        r#"
pub struct S;
/// doc
impl S {
    pub fn a(&self) -> i32 {
        2
    }
}
"#,
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
        r#"
/// doc
pub const N: i32 = 1;
"#,
        r#"
/// doc
pub const N: i32 = 2;
"#,
    );
    assert_eq!(c.kind, ItemKind::Const);
}

#[test]
fn module_scope_is_shallow_over_item_signatures() {
    let p = Project::new();
    let c = candidate_for(
        &p,
        r#"
//! module doc
pub fn f() {}
"#,
        r#"
//! module doc
pub fn f(a: i32) {}
"#,
    );
    assert_eq!(c.kind, ItemKind::Module);
    assert_eq!(c.item_path, "crate");
}

#[test]
fn module_scope_ignores_unrelated_body_edits() {
    let p = Project::new();
    p.write(
        "src/lib.rs",
        r#"
//! module doc
pub fn f() -> i32 {
    1
}
"#,
    );
    engine::init(p.path(), false).unwrap();

    p.write(
        "src/lib.rs",
        r#"
//! module doc
pub fn f() -> i32 {
    2
}
"#,
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
        r#"
pub fn f() {
    // note
    let x = 1;
}
"#,
        r#"
pub fn f() {
    // note
    let x = 2;
}
"#,
    );
    assert_eq!(c.kind, ItemKind::Free);
    assert!(c.id.contains("#free:"), "id was {}", c.id);
    assert!(c.old_body_text.contains("let x = 1"));
}

#[test]
fn comment_inside_a_while_loop_is_its_own_scope() {
    let p = Project::new();
    let c = candidate_for(
        &p,
        r#"
pub fn f(n: i32) -> i32 {
    // outer note
    let mut total = 0;
    while total < n {
        // inner note
        let step = 1;
        total += step;
    }
    total
}
"#,
        r#"
pub fn f(n: i32) -> i32 {
    // outer note
    let mut total = 0;
    while total < n {
        // inner note
        let step = 2;
        total += step;
    }
    total
}
"#,
    );
    assert_eq!(c.kind, ItemKind::Free);
    assert!(
        c.comment_text.contains("inner note"),
        "comment was {}",
        c.comment_text
    );
    assert!(c.old_body_text.contains("let step = 1"));
}

#[test]
fn free_comment_before_a_loop_does_not_react_to_changes_deep_inside_it() {
    let p = Project::new();
    p.write(
        "src/lib.rs",
        r#"
pub fn f(n: i32) -> i32 {
    // outer note
    let mut total = 0;
    while total < n {
        let step = 1;
        total += step;
    }
    total
}
"#,
    );
    engine::init(p.path(), false).unwrap();

    p.write(
        "src/lib.rs",
        r#"
pub fn f(n: i32) -> i32 {
    // outer note
    let mut total = 0;
    while total < n {
        let step = 2;
        total += step;
    }
    total
}
"#,
    );
    let outcome = engine::check(p.path()).unwrap();
    assert!(
        outcome.candidates.is_empty(),
        "the loop body has no comment of its own, got {:#?}",
        outcome.candidates
    );
}

#[test]
fn comment_inside_an_else_if_branch_is_tracked() {
    let p = Project::new();
    let c = candidate_for(
        &p,
        r#"
pub fn f(x: i32) -> i32 {
    if x < 0 {
        0
    } else if x < 10 {
        // small note
        x + 1
    } else {
        x
    }
}
"#,
        r#"
pub fn f(x: i32) -> i32 {
    if x < 0 {
        0
    } else if x < 10 {
        // small note
        x + 2
    } else {
        x
    }
}
"#,
    );
    assert_eq!(c.kind, ItemKind::Free);
    assert!(c.comment_text.contains("small note"));
}

#[test]
fn comment_inside_a_match_arm_block_is_tracked() {
    let p = Project::new();
    let c = candidate_for(
        &p,
        r#"
pub fn f(x: i32) -> i32 {
    match x {
        0 => 0,
        _ => {
            // nonzero note
            x + 1
        }
    }
}
"#,
        r#"
pub fn f(x: i32) -> i32 {
    match x {
        0 => 0,
        _ => {
            // nonzero note
            x + 2
        }
    }
}
"#,
    );
    assert_eq!(c.kind, ItemKind::Free);
    assert!(c.comment_text.contains("nonzero note"));
}

#[test]
fn trailing_side_comment_scopes_backward() {
    let p = Project::new();
    let c = candidate_for(
        &p,
        r#"
pub fn f() {
    let x = 1; // note
    let _ = x;
}
"#,
        r#"
pub fn f() {
    let x = 2; // note
    let _ = x;
}
"#,
    );
    assert_eq!(c.kind, ItemKind::Free);
    assert!(c.id.contains("#trailing:"), "id was {}", c.id);
    assert!(c.old_body_text.contains("let x = 1"));
    assert!(!c.old_body_text.contains("let _"));
}

#[test]
fn reformatting_alone_never_creates_a_candidate() {
    let p = Project::new();
    p.write(
        "src/lib.rs",
        r#"
/// doc
pub fn f(a: i32, b: i32) -> i32 {
    a + b
}
"#,
    );
    engine::init(p.path(), false).unwrap();

    p.write(
        "src/lib.rs",
        r#"
/// doc
pub fn f(a: i32, b: i32) -> i32 {
        a
        +
        b
}
"#,
    );
    let outcome = engine::check(p.path()).unwrap();
    assert!(outcome.candidates.is_empty());
    assert_eq!(outcome.unchanged, 1);
}
