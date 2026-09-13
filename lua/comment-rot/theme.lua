local M = {}

-- Curated palettes rather than linking to `DiffAdd`/`DiffDelete`/etc: a lot
-- of colorschemes tune those groups for real `'diff'` split-view (subtle,
-- meant to sit next to matching context), where they end up nearly
-- invisible in a plain floating buffer. Foreground + a light background
-- tint reads clearly regardless of colorscheme. `default = true` on every
-- group still lets a user override any of this from their own config.
local PALETTES = {
	dark = {
		diff_add_fg = "#7ee787",
		diff_add_bg = "#122b1a",
		diff_del_fg = "#ff7b72",
		diff_del_bg = "#341015",
		hunk = "#79c0ff",
		border = "#7aa2f7",
		quote = "#8b949e",
		muted = "#8b949e",
		key = "#e3b341",
		accent = "#c297ff",
		reason = "#e0965a",
	},
	light = {
		diff_add_fg = "#116329",
		diff_add_bg = "#dafbe1",
		diff_del_fg = "#82071e",
		diff_del_bg = "#ffebe9",
		hunk = "#0550ae",
		border = "#3b5bdb",
		quote = "#57606a",
		muted = "#57606a",
		key = "#9a6700",
		accent = "#8250df",
		reason = "#953800",
	},
}

local function hl(name, opts)
	vim.api.nvim_set_hl(0, name, vim.tbl_extend("force", { default = true }, opts))
end

function M.setup()
	local p = vim.o.background == "light" and PALETTES.light or PALETTES.dark

	hl("CommentRotDiffAdd", { fg = p.diff_add_fg, bg = p.diff_add_bg, bold = true })
	hl("CommentRotDiffDelete", { fg = p.diff_del_fg, bg = p.diff_del_bg, bold = true })
	hl("CommentRotDiffContext", { fg = p.muted })
	hl("CommentRotHunk", { fg = p.hunk, italic = true })
	hl("CommentRotBorder", { fg = p.border })
	hl("CommentRotQuote", { fg = p.quote, italic = true })
	hl("CommentRotMuted", { fg = p.muted })
	hl("CommentRotKey", { fg = p.key, bold = true })
	hl("CommentRotAccent", { fg = p.accent, bold = true })
	hl("CommentRotReason", { fg = p.reason, italic = true })
	hl("CommentRotProgressDone", { fg = p.accent })
	hl("CommentRotProgressPending", { fg = p.muted })
end

return M
