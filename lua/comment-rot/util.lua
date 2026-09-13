local M = {}

--- Walks up from `start` looking for an existing `.rot` baseline first,
--- then a `.git` directory, falling back to cwd. `.rot` is checked first
--- so a project that keeps its baseline above cwd (e.g. run from a
--- subcrate) is still found once initialized.
function M.find_root(start)
	start = start or vim.fn.expand("%:p:h")
	if start == "" then
		start = vim.fn.getcwd()
	end

	local rot_dir = vim.fs.find(".rot", { path = start, upward = true, type = "directory" })[1]
	if rot_dir then
		return vim.fs.dirname(rot_dir)
	end

	local git_dir = vim.fs.find(".git", { path = start, upward = true })[1]
	if git_dir then
		return vim.fs.dirname(git_dir)
	end

	return vim.fn.getcwd()
end

function M.notify(msg, level)
	vim.notify("[comment-rot] " .. msg, level or vim.log.levels.INFO)
end

--- PascalCase `ItemKind` (as serialized by the CLI's JSON output) to the
--- same lowercase label the CLI's own interactive UI prints.
M.kind_labels = {
	Function = "function",
	Struct = "struct",
	Enum = "enum",
	Union = "union",
	Trait = "trait",
	Impl = "impl block",
	Const = "const",
	Static = "static",
	TypeAlias = "type alias",
	Module = "module",
	Field = "field",
	Variant = "variant",
	Macro = "macro",
	Extern = "extern block",
	Free = "code",
}

function M.kind_label(kind)
	return M.kind_labels[kind] or kind
end

-- Colors the kind badge with the *standard* syntax groups every colorscheme
-- already defines, instead of inventing our own - a function shows up in
-- whatever blue/yellow/etc a theme uses for `Function`, so the badge always
-- matches the rest of the user's setup.
local kind_hl_groups = {
	Function = "Function",
	Struct = "Structure",
	Enum = "Structure",
	Union = "Structure",
	Trait = "Type",
	Impl = "Type",
	Const = "Constant",
	Static = "Constant",
	TypeAlias = "Type",
	Module = "Include",
	Field = "Identifier",
	Variant = "Identifier",
	Macro = "Macro",
	Extern = "StorageClass",
	Free = "Comment",
}

function M.kind_hl(kind)
	return kind_hl_groups[kind] or "Normal"
end

--- Finds which line in `abs_path` holds `comment_text` (its first
--- non-empty line, since that's stable across single- and multi-line
--- comments alike), falling back to `default_line` if the file is
--- unreadable or the text isn't found - e.g. `KnownIssue` doesn't carry a
--- line number at all, and any candidate's reported line can already be
--- stale by the time something acts on it if the buffer has unsaved edits.
function M.locate_comment_line(abs_path, comment_text, default_line)
	local anchor = nil
	for _, l in ipairs(vim.split(comment_text or "", "\n", { trimempty = true })) do
		anchor = vim.trim(l)
		break
	end
	if not anchor or anchor == "" then
		return default_line
	end

	local ok, lines = pcall(vim.fn.readfile, abs_path)
	if not ok then
		return default_line
	end

	for i, l in ipairs(lines) do
		if l:find(anchor, 1, true) then
			return i
		end
	end

	return default_line
end

return M
