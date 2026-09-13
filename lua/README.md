# comment-rot.nvim

Neovim frontend for [comment-rot](../README.md). Shells out to the `rot`
binary's `--json` interface

Requires Neovim 0.10+ (uses `vim.system` and `vim.json`).

## Install (LazyVim)

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

For keymaps, you can use `vim.keymap.set` directly too

```lua
vim.keymap.set("n", "<leader>rc", "<cmd>RotCheck<cr>", { desc = "Rot check" })
```

## Commands

- `:RotInit` / `:RotInit!`: create a baseline (`!` forces a from-scratch rebuild).
- `:RotCheck`: scan for stale comments and review them one at a time.
  - `y` comment is still accurate, move on
  - `n` comment is stale, flag it as a known issue for later (`:RotStatus`)
  - `g` jump to the code instead
  - `q` / `<Esc>` — stop early. Verdicts already given (`y`/`n`) are still
    applied.
- `:RotStatus`: list confirmed-stale comments not yet fixed in the quickfix
  list
  - `y` re-confirm the comment as accurate, clearing the issue
  - `n` re-flag it as stale, refreshing the code it's compared against
  - `r` dismiss the issue without changing the baseline
- `:RotResolve [id]`: clear a known issue.

## Colors

`:RotCheck`'s diff and UI accents use their own highlight groups
(see `lua/comment-rot/theme.lua`). Every group is set with `default = true`,
so overriding one yourself (`vim.api.nvim_set_hl(0, "CommentRotDiffAdd", {...})`)
still takes precedence.

## Known limitations

- `:RotStatus`'s line numbers are found by searching the file for the
  flagged comment's text (the CLI's `status --json` doesn't report a line
  itself), so an entry can point at the wrong line if that exact comment
  text now appears more than once in the file, or fall back to line 1 if
  the comment was since deleted entirely.

## About

This plugin was fully created with AI. I do not know Lua or Neovim plugin
development.
