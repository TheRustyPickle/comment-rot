use anyhow::{Context, Result};
use std::collections::HashMap;
use tree_sitter::{Node, Parser};

use crate::hashing::{normalize_for_hash, short_anchor};
use crate::model::{CommentKind, ItemKind, ScopedItem};

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
/// `item_path`
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

/// Walks `node`'s subtree looking for every block-introducing construct
/// `while`/`if`/`for`/`loop`/`match`, closures, and `unsafe`/`async`/bare
/// blocks
fn find_nested_blocks<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
    if is_target_kind(node.kind()) {
        return;
    }

    match node.kind() {
        "while_expression" | "for_expression" | "loop_expression" => {
            if let Some(body) = node.child_by_field_name("body") {
                out.push(body);
            }

            for field in ["condition", "value", "pattern"] {
                if let Some(n) = node.child_by_field_name(field) {
                    find_nested_blocks(n, out);
                }
            }
        }

        "if_expression" => {
            if let Some(consequence) = node.child_by_field_name("consequence") {
                out.push(consequence);
            }

            if let Some(condition) = node.child_by_field_name("condition") {
                find_nested_blocks(condition, out);
            }

            if let Some(alt) = node.child_by_field_name("alternative") {
                let mut cursor = alt.walk();
                for child in alt.named_children(&mut cursor) {
                    if child.kind() == "block" {
                        out.push(child);
                    } else {
                        find_nested_blocks(child, out); // a chained `else if`
                    }
                }
            }
        }

        "match_expression" => {
            if let Some(value) = node.child_by_field_name("value") {
                find_nested_blocks(value, out);
            }

            if let Some(body) = node.child_by_field_name("body") {
                let mut cursor = body.walk();

                for arm in body.named_children(&mut cursor) {
                    if arm.kind() != "match_arm" {
                        continue;
                    }

                    if let Some(value) = arm.child_by_field_name("value") {
                        if value.kind() == "block" {
                            out.push(value);
                        } else {
                            find_nested_blocks(value, out);
                        }
                    }
                }
            }
        }

        "closure_expression" => {
            if let Some(body) = node.child_by_field_name("body") {
                if body.kind() == "block" {
                    out.push(body);
                } else {
                    find_nested_blocks(body, out); // unbraced closure body
                }
            }
        }

        "unsafe_block" | "async_block" => {
            let mut cursor = node.walk();

            if let Some(block) = node
                .named_children(&mut cursor)
                .find(|c| c.kind() == "block")
            {
                out.push(block);
            }
        }

        "block" => out.push(node),
        _ => {
            let mut cursor = node.walk();

            for child in node.named_children(&mut cursor) {
                find_nested_blocks(child, out);
            }
        }
    }
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

fn is_target_kind(kind: &str) -> bool {
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

    node.child_by_field_name("name").map_or_else(
        || "<anonymous>".to_string(),
        |n| src[n.byte_range()].to_string(),
    )
}

fn line_of(node: &Node) -> usize {
    node.start_position().row + 1
}

/// Collects the byte ranges of every comment nested anywhere under `node`
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
/// This is how "body" text ignores nested comments
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
/// newline. Used to build a shallow signature for containers whose doc
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

/// A single span node's text with the interior of every
/// nested block found within it collapsed away
fn collapse_nested_blocks(node: &Node, src: &str) -> String {
    let mut cuts = Vec::new();
    collect_comment_ranges(*node, &mut cuts);

    let mut blocks = Vec::new();

    find_nested_blocks(*node, &mut blocks);

    for b in &blocks {
        if b.end_byte() > b.start_byte() + 2 {
            cuts.push((b.start_byte() + 1, b.end_byte() - 1));
        }
    }

    cuts.sort_by_key(|r| r.0);

    let mut result = String::new();
    let mut pos = node.start_byte();

    for (s, e) in cuts {
        if s > pos {
            result.push_str(&src[pos..s]);
        }
        pos = pos.max(e);
    }

    if pos < node.end_byte() {
        result.push_str(&src[pos..node.end_byte()]);
    }

    result
}

/// A free comment's forward-scoped body: each statement's own text with
/// any nested block collapsed away (see `collapse_nested_blocks`).
fn free_span_text(span: &[Node], src: &str) -> String {
    span.iter()
        .map(|n| collapse_nested_blocks(n, src))
        .filter(|s| !s.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn item_body_text(attrs: &[Node], node: &Node, src: &str) -> String {
    let attrs_text = attrs
        .iter()
        .map(|a| strip_comments_node(*a, src))
        .collect::<Vec<_>>()
        .join("\n");

    let own = match node.kind() {
        "mod_item" | "trait_item" | "impl_item" => match node.child_by_field_name("body") {
            Some(body) => shallow_container_signature(&direct_children(body), src),
            None => strip_comments_node(*node, src),
        },
        _ => strip_comments_node(*node, src),
    };

    if attrs_text.is_empty() {
        own
    } else {
        format!("{attrs_text}\n{own}")
    }
}

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

    if n == 0 { base } else { format!("{base}:{n}") }
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

    let item = ScopedItem {
        id: enjoin(file, &item_path),
        kind: ItemKind::Free,
        file: file.to_string(),
        item_path,
        comment_kind,
        comment_text: text,
        body_text: body,
        line: line_of(&group[0]),
    };

    out.push(item);
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
    let body = free_span_text(span, src);

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

    let item = ScopedItem {
        id: enjoin(file, &item_path),
        kind: ItemKind::Free,
        file: file.to_string(),
        item_path,
        comment_kind,
        comment_text: text,
        body_text: body,
        line: line_of(&group[0]),
    };

    out.push(item);
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
            && is_target_kind(next.kind())
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
                body_text: item_body_text(&children[i..item_idx], next, src),
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

    // Recurse into nested containers
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
            _ => {
                // Not a named item. Look anywhere in this statement for a
                // nested block and recurse into each one
                let mut blocks = Vec::new();

                find_nested_blocks(*child, &mut blocks);

                for block in blocks {
                    let sub_children = direct_children(block);
                    scan_container(
                        &sub_children,
                        src,
                        file,
                        enclosing_path,
                        enclosing_kind,
                        dedup,
                        out,
                    );
                }

                continue;
            }
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
