<!-- rumdl-disable-file no-inline-html -->
<div align="center"> <!-- rumdl-disable-line first-line-heading -->

# Comment Rot

<a href="https://wakatime.com/@RustyPickle"><img src="https://wakatime.com/badge/github/TheRustyPickle/comment-rot.svg" alt="wakatime"></a>
<a href="https://crates.io/crates/comment-rot">
<img src="https://img.shields.io/crates/v/comment-rot.svg?style=flat-square&logo=rust&color=orange"/></a>
<a href="https://github.com/TheRustyPickle/comment-rot/releases/latest">
<img src="https://img.shields.io/github/v/release/TheRustyPickle/comment-rot?style=flat-square&logo=github&color=orange"/></a>
<a href="https://crates.io/crates/comment-rot">
<img src="https://img.shields.io/crates/d/comment-rot?style=flat-square"/></a>

</div>

Finds comments that went stale when the code next to them changed.

Rot snapshots each comment alongside the code it's attached to. On the next
scan, code changed but the comment didn't gets flagged for review.
Comments are tracked by what they document, not by line number, so unrelated
edits never cause false positives.

## Install

**1. Run from Source Code:**

* Clone the repository
`
git clone https://github.com/TheRustyPickle/comment-rot
`
* Run with Cargo
`
cargo run --release
`

**2. Run the Latest Release:**

* Download the latest executable from [Release](https://github.com/TheRustyPickle/comment-rot/releases/latest).
* Unzip the archive and run the binary.

**3. Install from Cargo:**

* Install with `cargo install comment-rot`
* Run with the command `rot`

## Usage

```sh
rot init              # take a baseline of the project
rot check             # find comments that need review
rot confirm           # apply verdicts (reads JSON from stdin)
rot status            # list issues flagged but not yet fixed
rot resolve <id>      # dismiss a known issue
```

## Neovim plugin

A Neovim frontend lives in [`lua/`](lua)

```lua
return {
    "https://github.com/TheRustyPickle/comment-rot/",
    build = "cargo build --release",
    keys = {
        { "<leader>ri", "<cmd>RotInit<cr>", desc = "Rot init" },
        { "<leader>rc", "<cmd>RotCheck<cr>", desc = "Rot check" },
        { "<leader>rs", "<cmd>RotStatus<cr>", desc = "Rot status" },
        { "<leader>rr", "<cmd>RotResolve<cr>", desc = "Rot resolve" },
    },
    opts = {},
}
```

See [`lua/README.md`](lua/README.md) for commands, keymaps, and colors.

## Known limitations

* Comments nested inside a single expression (mid method-chain
  `.iter() // note`) aren't tracked. Only comments sitting between whole
  statements/items are.
* Item paths are built from each file's own AST, not Rust's real module
  graph. Two files can't currently be resolved as the same logical module
  via `mod` declarations elsewhere.
* Only supports Rust files

## Contributing

Found a bug or an edge case? Open an issue. Fixes and improvements are welcome
as pull requests.

## License

This project is under [MIT](LICENSE).
