use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::hashing::hash_str;

/// Kind of syntax construct a scope entry is attached to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ItemKind {
    Function,
    Struct,
    Enum,
    Union,
    Trait,
    Impl,
    Const,
    Static,
    TypeAlias,
    Module,
    Field,
    Variant,
    Macro,
    Free,
}

impl ItemKind {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            ItemKind::Function => "function",
            ItemKind::Struct => "struct",
            ItemKind::Enum => "enum",
            ItemKind::Union => "union",
            ItemKind::Trait => "trait",
            ItemKind::Impl => "impl block",
            ItemKind::Const => "const",
            ItemKind::Static => "static",
            ItemKind::TypeAlias => "type alias",
            ItemKind::Module => "module",
            ItemKind::Field => "field",
            ItemKind::Variant => "variant",
            ItemKind::Macro => "macro",
            ItemKind::Free => "code",
        }
    }
}

/// Where a comment's text came from, syntactically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CommentKind {
    /// `///` or `/** */`
    OuterDoc,
    /// `//!` or `/*! */`
    InnerDoc,
    /// Plain `//` line comment.
    Line,
    /// Plain `/* */` block comment.
    Block,
}

#[derive(Debug, Clone)]
pub struct ScopedItem {
    pub id: String,
    pub kind: ItemKind,
    pub file: String,
    pub item_path: String,
    pub comment_kind: CommentKind,
    pub comment_text: String,
    pub body_text: String,
    pub line: usize,
}

impl ScopedItem {
    #[must_use]
    pub fn comment_hash(&self) -> String {
        hash_str(&self.comment_text)
    }

    #[must_use]
    pub fn body_hash(&self) -> String {
        hash_str(&self.body_text)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotEntry {
    pub kind: ItemKind,
    pub file: String,
    pub item_path: String,
    pub comment_kind: CommentKind,
    pub comment_text: String,
    pub comment_hash: String,
    pub body_text: String,
    pub body_hash: String,
}

impl From<&ScopedItem> for SnapshotEntry {
    fn from(item: &ScopedItem) -> Self {
        SnapshotEntry {
            kind: item.kind,
            file: item.file.clone(),
            item_path: item.item_path.clone(),
            comment_kind: item.comment_kind,
            comment_text: item.comment_text.clone(),
            comment_hash: item.comment_hash(),
            body_text: item.body_text.clone(),
            body_hash: item.body_hash(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: u32,
    pub generated_at: u64,
    pub entries: BTreeMap<String, SnapshotEntry>,
}

impl Snapshot {
    pub const CURRENT_VERSION: u32 = 1;

    #[must_use]
    pub fn new() -> Self {
        Snapshot {
            version: Self::CURRENT_VERSION,
            generated_at: now_unix(),
            entries: BTreeMap::default(),
        }
    }
}

impl Default for Snapshot {
    fn default() -> Self {
        Self::new()
    }
}

/// A stale-comment candidate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub id: String,
    pub kind: ItemKind,
    pub file: String,
    pub item_path: String,
    pub line: usize,
    pub comment_kind: CommentKind,
    pub comment_text: String,
    pub old_body_text: String,
    pub new_body_text: String,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    /// The comment is still accurate
    Yes,
    /// The comment is stale
    No,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfirmInput {
    pub id: String,
    pub verdict: Verdict,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownIssue {
    pub kind: ItemKind,
    pub file: String,
    pub item_path: String,
    pub flagged_at: u64,
    pub comment_text: String,
    pub old_body_text: String,
    pub new_body_text: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownIssues {
    pub version: u32,
    pub issues: BTreeMap<String, KnownIssue>,
}

impl KnownIssues {
    pub const CURRENT_VERSION: u32 = 1;

    #[must_use]
    pub fn new() -> Self {
        KnownIssues {
            version: Self::CURRENT_VERSION,
            issues: BTreeMap::default(),
        }
    }
}

impl Default for KnownIssues {
    fn default() -> Self {
        Self::new()
    }
}

#[must_use]
pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}
