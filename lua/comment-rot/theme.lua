local M = {}

--- Resolves `name` all the way through to its actual colors rather than
--- returning `{ link = name }` - so the result can be merged with extra
--- attributes (bold, italic) below, which `link` alone can't be combined
--- with (nvim_set_hl ignores other fields once `link` is present).
local function resolve(name)
	return vim.api.nvim_get_hl(0, { name = name, link = false })
end

local function hl(name, opts)
	vim.api.nvim_set_hl(0, name, vim.tbl_extend("force", { default = true }, opts))
end

--- Every group here is derived from the active colorscheme's own
--- `Diagnostic*`/`Comment`/`FloatBorder` groups rather than a hardcoded
--- palette, so the UI matches whatever theme is active with no extra
--- setup. `Diagnostic*` specifically (not `DiffAdd`/`DiffDelete`) because
--- diagnostics are tuned to be attention-grabbing in every colorscheme -
--- diff groups are tuned for real split-view and often end up washed out
--- in a plain floating buffer. A colorscheme that leaves these dull is a
--- rarer problem than that was.
function M.setup()
	hl("CommentRotDiffAdd", vim.tbl_extend("force", resolve("DiagnosticOk"), { bold = true }))
	hl("CommentRotDiffDelete", vim.tbl_extend("force", resolve("DiagnosticError"), { bold = true }))
	hl("CommentRotDiffContext", resolve("Comment"))
	hl("CommentRotHunk", vim.tbl_extend("force", resolve("DiagnosticHint"), { italic = true }))
	hl("CommentRotBorder", resolve("FloatBorder"))
	hl("CommentRotQuote", vim.tbl_extend("force", resolve("Comment"), { italic = true }))
	hl("CommentRotMuted", resolve("Comment"))
	hl("CommentRotKey", vim.tbl_extend("force", resolve("DiagnosticHint"), { bold = true }))
	hl("CommentRotAccent", vim.tbl_extend("force", resolve("DiagnosticInfo"), { bold = true }))
	hl("CommentRotReason", vim.tbl_extend("force", resolve("DiagnosticWarn"), { italic = true }))
	hl("CommentRotProgressDone", resolve("DiagnosticInfo"))
	hl("CommentRotProgressPending", resolve("Comment"))
end

return M
