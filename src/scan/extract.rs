use crate::hashing::{normalize_for_hash, short_anchor};
use crate::model::{CommentKind, ItemKind, ScopedItem};
use anyhow::{Context, Result};
use std::collections::HashMap;
use tree_sitter::{Node, Parser};

/// Parses one Rust source file and extracts every doc-comment
/// and regular-comment
///
/// A doc comment (`///`, `//!`, `/** */`, `/*! */`)
/// owns whatever item it immediately precedes (or, for `//!`, the
/// enclosing container itself). a regular comment (`//`, `/* */`) never
/// owns an item. Its scope is the code it actually comments on. The
/// single preceding statement if it trails on the same line, or the code
/// that follows
pub fn extract_file(file_rel: &str, src: &str) -> Result<Vec<ScopedItem>> {
    let mut parser = Parser::new();

    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .context("loading rust grammar")?;

    let tree = parser
        .parse(src, None)
        .context("tree-sitter failed to produce a parse tree")?;

    let root = tree.root_node();
    let mut out = Vec::new();
    let mut dedup = HashMap::new();

    let children = direct_children(root);

    scan_container(
        &children,
        src,
        file_rel,
        "crate",
        ItemKind::Module,
        &mut dedup,
        &mut out,
    );
    Ok(out)
}

/// Every entry's `id` is qualified by file, on top of its human-readable
/// `item_path` - two files can otherwise legitimately have the same
/// module-relative path (e.g. two binaries each with their own crate
/// root), and without this they'd collide in the snapshot map.
fn enjoin(file: &str, item_path: &str) -> String {
    format!("{file}|{item_path}")
}

fn direct_children(node: Node) -> Vec<Node> {
    let mut cursor = node.walk();
    node.children(&mut cursor).collect()
}

fn is_comment(node: &Node) -> bool {
    matches!(node.kind(), "line_comment" | "block_comment")
}

fn is_doc_comment(node: &Node) -> bool {
    node.child_by_field_name("outer").is_some() || node.child_by_field_name("inner").is_some()
}

fn is_inner_doc(node: &Node) -> bool {
    node.child_by_field_name("inner").is_some()
}

fn doc_text(node: &Node, src: &str) -> String {
    if let Some(d) = node.child_by_field_name("doc") {
        src[d.byte_range()].to_string()
    } else {
        src[node.byte_range()].to_string()
    }
}

fn is_item_like(kind: &str) -> bool {
    matches!(
        kind,
        "function_item"
            | "function_signature_item"
            | "struct_item"
            | "enum_item"
            | "union_item"
            | "trait_item"
            | "impl_item"
            | "const_item"
            | "static_item"
            | "type_item"
            | "mod_item"
            | "field_declaration"
            | "enum_variant"
            | "macro_definition"
            | "associated_type"
            | "foreign_mod_item"
    )
}

fn kind_of(node_kind: &str) -> ItemKind {
    match node_kind {
        "function_item" | "function_signature_item" => ItemKind::Function,
        "struct_item" => ItemKind::Struct,
        "enum_item" => ItemKind::Enum,
        "union_item" => ItemKind::Union,
        "trait_item" => ItemKind::Trait,
        "impl_item" => ItemKind::Impl,
        "const_item" => ItemKind::Const,
        "static_item" => ItemKind::Static,
        "type_item" => ItemKind::TypeAlias,
        "mod_item" => ItemKind::Module,
        "field_declaration" => ItemKind::Field,
        "enum_variant" => ItemKind::Variant,
        "macro_definition" => ItemKind::Macro,
        "associated_type" => ItemKind::TypeAlias,
        "foreign_mod_item" => ItemKind::Extern,
        _ => ItemKind::Free,
    }
}

fn item_name(node: &Node, src: &str) -> String {
    if node.kind() == "impl_item" {
        let ty = node
            .child_by_field_name("type")
            .map(|n| src[n.byte_range()].to_string());

        let tr = node
            .child_by_field_name("trait")
            .map(|n| src[n.byte_range()].to_string());

        return match (tr, ty) {
            (Some(tr), Some(ty)) => format!("impl {tr} for {ty}"),
            (None, Some(ty)) => format!("impl {ty}"),
            _ => "impl".to_string(),
        };
    }

    if node.kind() == "foreign_mod_item" {
        let mut cursor = node.walk();
        return node
            .children(&mut cursor)
            .find(|c| c.kind() == "extern_modifier")
            .map_or_else(|| "extern".to_string(), |n| src[n.byte_range()].to_string());
    }

    node.child_by_field_name("name").map_or_else(|| "<anonymous>".to_string(), |n| src[n.byte_range()].to_string())
}

fn line_of(node: &Node) -> usize {
    node.start_position().row + 1
}

/// Collects the byte ranges of every comment nested anywhere under `node`
/// (not recursing into a comment's own internals).
fn collect_comment_ranges(node: Node, out: &mut Vec<(usize, usize)>) {
    if is_comment(&node) {
        out.push((node.start_byte(), node.end_byte()));
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_comment_ranges(child, out);
    }
}

/// Reconstructs the source text of `[start, end)` with every comment inside
/// `nodes` spliced out - this is how "body" text ignores nested comments
/// entirely, so an inner comment edit never changes an outer hash.
fn strip_comments_range(start: usize, end: usize, nodes: &[Node], src: &str) -> String {
    let mut ranges = Vec::new();
    for n in nodes {
        collect_comment_ranges(*n, &mut ranges);
    }
    ranges.sort_by_key(|r| r.0);
    let mut result = String::new();
    let mut pos = start;
    for (s, e) in ranges {
        if s > pos {
            result.push_str(&src[pos..s]);
        }
        pos = pos.max(e);
    }
    if pos < end {
        result.push_str(&src[pos..end]);
    }
    result
}

fn strip_comments_node(node: Node, src: &str) -> String {
    strip_comments_range(node.start_byte(), node.end_byte(), &[node], src)
}

/// First line of an item's (comment-stripped) text, up to `{`, `;`, or a
/// newline - used to build a shallow signature for containers whose doc
/// comment should track "what's in here" rather than "how it's implemented".
fn shallow_signature(node: &Node, src: &str) -> String {
    let full = strip_comments_node(*node, src);
    let cut = full.find(['{', ';', '\n']).unwrap_or(full.len());
    full[..cut].trim().to_string()
}

fn shallow_container_signature(children: &[Node], src: &str) -> String {
    children
        .iter()
        .filter(|n| !is_comment(n))
        .map(|n| shallow_signature(n, src))
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Body text to hash for an outer-doc-commented item. Modules, traits, and
/// impl blocks are hashed shallowly (their member signature list) so an
/// edit to one method's *implementation* doesn't invalidate a doc comment
/// that only describes what the container holds; everything else (notably
/// functions) is hashed against its full comment-stripped content.
fn item_body_text(node: &Node, src: &str) -> String {
    match node.kind() {
        "mod_item" | "trait_item" | "impl_item" => match node.child_by_field_name("body") {
            Some(body) => shallow_container_signature(&direct_children(body), src),
            None => strip_comments_node(*node, src),
        },
        _ => strip_comments_node(*node, src),
    }
}

/// Builds the `item_path` for a free/trailing comment scope. Identity is
/// anchored on the *comment's own text*, not the code it describes - the
/// whole point is to keep the id stable while the body drifts out of sync,
/// so a candidate can actually surface. If the comment itself changes,
/// that's a new id (old one pruned, new one added), which matches the
/// "comment changed too = assumed intentional" rule used everywhere else.
/// `dedup` disambiguates repeated identical comment text within the same
/// enclosing scope (e.g. two `// TODO` comments in one function) without
/// resorting to line numbers.
fn free_item_path(
    enclosing_path: &str,
    tag: &str,
    comment_text: &str,
    dedup: &mut HashMap<String, usize>,
) -> String {
    let anchor = short_anchor(&normalize_for_hash(comment_text));
    let base = format!("{enclosing_path}#{tag}:{anchor}");
    let slot = dedup.entry(base.clone()).or_insert(0);
    let n = *slot;
    *slot += 1;
    if n == 0 {
        base
    } else {
        format!("{base}:{n}")
    }
}

/// A side comment (`stmt(); // like this`)
fn handle_trailing_group(
    group: &[Node],
    preceding: &Node,
    src: &str,
    file: &str,
    enclosing_path: &str,
    dedup: &mut HashMap<String, usize>,
    out: &mut Vec<ScopedItem>,
) {
    let text = group // test trailing here again
        .iter()
        .map(|c| src[c.byte_range()].to_string())
        .collect::<Vec<_>>()
        .join("\n");

    let body = strip_comments_node(*preceding, src);

    if body.trim().is_empty() {
        return;
    }

    let item_path = free_item_path(enclosing_path, "trailing", &text, dedup);
    let last = group.last().unwrap();
    let comment_kind = if last.kind() == "block_comment" {
        CommentKind::Block
    } else {
        CommentKind::Line
    };
    out.push(ScopedItem {
        id: enjoin(file, &item_path),
        kind: ItemKind::Free,
        file: file.to_string(),
        item_path,
        comment_kind,
        comment_text: text,
        body_text: body,
        line: line_of(&group[0]),
    });
}

#[allow(clippy::too_many_arguments)]
fn handle_free_group(
    group: &[Node],
    children: &[Node],
    after_idx: usize,
    src: &str,
    file: &str,
    enclosing_path: &str,
    dedup: &mut HashMap<String, usize>,
    out: &mut Vec<ScopedItem>,
) {
    let text = group
        .iter()
        .map(|c| src[c.byte_range()].to_string())
        .collect::<Vec<_>>()
        .join("\n");

    let n = children.len();
    let mut span_end = after_idx;
    while span_end < n && !is_comment(&children[span_end]) {
        span_end += 1;
    }
    if span_end == after_idx {
        return; // nothing follows before the next comment / end of container
    }
    let span = &children[after_idx..span_end];
    let start = span[0].start_byte();
    let end = span[span.len() - 1].end_byte();
    let body = strip_comments_range(start, end, span, src);
    if body.trim().is_empty() {
        return;
    }

    let item_path = free_item_path(enclosing_path, "free", &text, dedup);
    let last = group.last().unwrap();
    let comment_kind = if last.kind() == "block_comment" {
        CommentKind::Block
    } else {
        CommentKind::Line
    };
    out.push(ScopedItem {
        id: enjoin(file, &item_path),
        kind: ItemKind::Free,
        file: file.to_string(),
        item_path,
        comment_kind,
        comment_text: text,
        body_text: body,
        line: line_of(&group[0]),
    });
}

fn scan_container(
    children: &[Node],
    src: &str,
    file: &str,
    enclosing_path: &str,
    enclosing_kind: ItemKind,
    dedup: &mut HashMap<String, usize>,
    out: &mut Vec<ScopedItem>,
) {
    let n = children.len();
    let mut i = 0;

    // Check for `//!`/`/*! */` comments at the start, usually found at the top level of a file.
    let mut lead = 0;

    while lead < n && is_comment(&children[lead]) && is_inner_doc(&children[lead]) {
        lead += 1;
    }

    if lead > 0 {
        let group = &children[0..lead];

        let text = group
            .iter()
            .map(|c| doc_text(c, src))
            .collect::<Vec<_>>()
            .join("\n");

        let body = shallow_container_signature(&children[lead..], src);

        let item = ScopedItem {
            id: enjoin(file, enclosing_path),
            kind: enclosing_kind,
            file: file.to_string(),
            item_path: enclosing_path.to_string(),
            comment_kind: CommentKind::InnerDoc,
            comment_text: text,
            body_text: body,
            line: line_of(&group[0]),
        };

        out.push(item);

        i = lead;
    }

    while i < n {
        if !is_comment(&children[i]) {
            i += 1;
            continue;
        }

        // None comments ignored. Starting position is the first comment.
        let group_start = i;

        while i < n && is_comment(&children[i]) {
            i += 1;
        }

        // Comment location captured
        let group = &children[group_start..i];
        let last = group.last().unwrap();

        // Attributes `#[arg(long)]`, `#[derive(...)]` etc are siblings of
        // the item they annotate, not part of it. A doc comment can have
        // one or more of these sitting between it
        let mut item_idx = i;
        while item_idx < n && children[item_idx].kind() == "attribute_item" {
            item_idx += 1;
        }

        if is_doc_comment(last)
            && !is_inner_doc(last)
            && let Some(next) = children.get(item_idx)
            && is_item_like(next.kind())
        {
            let text = group
                .iter()
                .filter(|c| is_doc_comment(c))
                .map(|c| doc_text(c, src))
                .collect::<Vec<_>>()
                .join("\n");

            let name = item_name(next, src);
            let path = format!("{enclosing_path}::{name}");

            let item = ScopedItem {
                id: enjoin(file, &path),
                kind: kind_of(next.kind()),
                file: file.to_string(),
                item_path: path,
                comment_kind: CommentKind::OuterDoc,
                comment_text: text,
                body_text: item_body_text(next, src),
                line: line_of(&group[0]),
            };

            out.push(item);

            i = item_idx + 1;
            continue;
        }

        // Not a doc comment attached to the item
        let trailing = group_start > 0
            && children[group_start - 1].end_position().row == group[0].start_position().row;

        if trailing {
            handle_trailing_group(
                group,
                &children[group_start - 1],
                src,
                file,
                enclosing_path,
                dedup,
                out,
            );
        } else {
            handle_free_group(group, children, i, src, file, enclosing_path, dedup, out);
        }
    }

    // Recurse into nested containers regardless of whether they had a doc comment.
    for child in children {
        let (sub_kind, body_field) = match child.kind() {
            "mod_item" => (ItemKind::Module, "body"),
            "struct_item" => (ItemKind::Struct, "body"),
            "union_item" => (ItemKind::Union, "body"),
            "enum_item" => (ItemKind::Enum, "body"),
            "trait_item" => (ItemKind::Trait, "body"),
            "impl_item" => (ItemKind::Impl, "body"),
            "function_item" => (ItemKind::Function, "body"),
            "enum_variant" => (ItemKind::Variant, "body"),
            "foreign_mod_item" => (ItemKind::Extern, "body"),
            _ => continue,
        };

        let Some(body) = child.child_by_field_name(body_field) else {
            continue;
        };

        if matches!(child.kind(), "struct_item" | "union_item" | "enum_variant")
            && body.kind() != "field_declaration_list"
        {
            continue; // tuple struct/union/variant - no named members to recurse into
        }

        let name = item_name(child, src);
        let path = format!("{enclosing_path}::{name}");
        let sub_children = direct_children(body);

        scan_container(&sub_children, src, file, &path, sub_kind, dedup, out);
    }
}
