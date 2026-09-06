<div align="center">

# Comment Rot

</div>

Finds comments that went stale when the code next to them changed.

Rot snapshots each comment alongside the code it's attached to. On the next
scan, code changed but the comment didn't gets flagged for review.
Both changed together are assumed intentional, skipped silently.
Comments are tracked by what they document, not by line number, so unrelated
edits never cause false positives.

## Install

```sh
cargo install --path .
```

## Usage

```sh
rot init              # take a baseline of the project
rot check             # find comments that need review
rot confirm           # apply verdicts (reads JSON from stdin)
rot status            # list issues flagged but not yet fixed
rot resolve <id>      # dismiss a known issue
```

`check` and `status` prompt interactively by default. Add `--json` to either
for machine-readable output instead which is meant for driving Rot from an editor
or another tool.

## Known limitations

- Comments nested inside a single expression (mid method-chain
  `.iter() // note`) aren't tracked. Only comments sitting between whole
  statements/items are.
- Item paths are built from each file's own AST, not Rust's real module
  graph. Two files can't currently be resolved as the same logical module
  via `mod` declarations elsewhere.

## Contributing

Found a bug or an edge case? Open an issue. Fixes and improvements are welcome
as pull requests.

## License

This project is under [MIT](LICENSE).
