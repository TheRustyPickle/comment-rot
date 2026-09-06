# Rot

A CLI that finds comments left behind by code changes.

Rot snapshots the pairing between every comment and the code it describes,
then flags any pairing where the code changed but the comment didn't

## How it works

1. **`rot init`** parses the project and records a baseline: for every
   comment it finds, the comment's text and the code it's attached to.
2. **`rot check`** re-scans the project and compares against that baseline:
   - Code changed, comment unchanged → flagged as a **candidate** needing a
     human look.
   - Both changed together → assumed intentional, silently folded into the
     new baseline.
   - Nothing changed, or the item is brand new → also folded in, no prompt.
3. You give each candidate a verdict:
   - **Yes**, the comment's still accurate → baseline moves on, done.
   - **No**, it's stale → recorded as a **known issue** (`rot status`) until
     you fix it or dismiss it with `rot resolve`.

Identity is never based on line numbers - an entry is tracked by its item
path (e.g. `crate::Foo::bar`) or, for a plain `//` comment, by an anchor
derived from the comment's own text. Unrelated edits shifting lines around
never trigger a false positive.

### What counts as "the comment" vs. "the code"

- A doc comment (`///`, `//!`, `/** */`, `/*! */`) belongs to the item it
  documents - a function, struct, field, enum variant, module, etc.
  Attributes (`#[derive(...)]`, `#[arg(...)]`, ...) between the doc comment
  and the item are transparent, so this works with derive-heavy code too.
- A regular comment (`//`, `/* */`) never "documents" an item. Its scope is
  whatever code it's actually sitting next to: the statement it trails on
  the same line (`stmt(); // like this`), or the code that follows it if
  it's on its own line.
- Container items (modules, traits, impl blocks) are tracked shallowly -
  their doc comment tracks the member signatures inside, not full method
  bodies, so refactoring a method's implementation doesn't flag the
  container's doc comment.

## Install

Not published to crates.io yet - build from source:

```sh
cargo install --path .
```

## Usage

```sh
rot init [--path .] [--force]      # take the first baseline
rot check [--path .] [--json]      # scan for stale-comment candidates
rot confirm [--path .]             # apply verdicts (JSON in via stdin)
rot status [--path .] [--json]     # list unresolved known issues
rot resolve [--path .] <id>        # manually clear a known issue
```

Interactive mode (`rot check` without `--json`) walks you through each
candidate with a colored diff and a yes/no prompt, then applies your
answers itself.

`--json` mode never prompts - it's meant for driving Rot from another tool:

```sh
$ rot check --json
{
  "candidates": [ { "id": "...", "kind": "Function", "comment_text": "...", ... } ],
  "added": 0, "updated": 2, "unchanged": 41, "pruned": 0, "issues_cleared": 0
}

$ echo '[{"id": "src/lib.rs|crate::add", "verdict": "no"}]' | rot confirm
{ "updated": 1, "flagged": 1, "skipped_missing": 0 }
```

This is the intended integration point for editor tooling (e.g. a Neovim
plugin): the engine (`rot::engine`, `rot::model`) has no UI code in it at
all, so any front end can drive it the same way the built-in interactive
and `--json` modes do.

### `.rot/`

`rot init` creates `.rot/snapshot.json` (the baseline) and
`.rot/known_issues.json` (unresolved verdicts). Commit both so your whole
team - and CI - shares the same baseline instead of everyone re-flagging
the same comments as "new."

## Known limitations

- Comments nested inside a single expression (e.g. mid method-chain -
  `.iter() // note`) aren't tracked. Only comments sitting between whole
  statements/items are - describing "just this one call in a chain" isn't
  a scope Rot models.
- Item paths are built from the file's own AST, not Rust's real module
  graph - two files can't currently be resolved as the same logical module
  via `mod` declarations elsewhere.

## Development

```sh
cargo nextest run        # tests (tests/ + a few inline unit tests)
cargo fmt
cargo clippy --all-targets
```

## License

MIT - see [LICENSE](LICENSE).
