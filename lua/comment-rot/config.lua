local M = {}

M.defaults = {
	-- Path (or bare name resolved via $PATH) to the `rot` executable.
	-- Left nil to auto-detect: a `target/release/rot` built next to this
	-- plugin's own files (see the `build` step in the README) takes
	-- priority, falling back to plain `rot` on $PATH.
	bin = nil,
}

M.options = vim.deepcopy(M.defaults)

--- This file lives at `<plugin root>/lua/comment-rot/config.lua`, so three
--- `:h`s off its own path is the plugin root - the same directory a local
--- `dir = "..."` lazy.nvim spec builds in, letting us find a binary that
--- spec's `build` step produced without the user having to hardcode a path.
local function plugin_root()
	local source = debug.getinfo(1, "S").source
	local file = source:sub(1, 1) == "@" and source:sub(2) or source
	return vim.fn.fnamemodify(file, ":h:h:h")
end

--- The `target/release/rot` this plugin's own `build` step would have
--- produced, or nil if that hasn't happened (yet, or ever - e.g. a global
--- `cargo install` setup instead). Kept separate from the `$PATH` fallback
--- below so the two cases can be told apart for the warning.
local function local_release_bin()
	local root = plugin_root()
	for _, name in ipairs({ "rot", "rot.exe" }) do
		local candidate = vim.fs.joinpath(root, "target", "release", name)
		if vim.uv.fs_stat(candidate) then
			return candidate
		end
	end
	return nil
end

function M.setup(opts)
	M.options = vim.tbl_deep_extend("force", vim.deepcopy(M.defaults), opts or {})

	if not M.options.bin then
		local local_bin = local_release_bin()
		if local_bin then
			M.options.bin = local_bin
		else
			M.options.bin = "rot"
			-- A `$PATH`-resolved `rot` (e.g. an old `cargo install`) can
			-- silently be a different build than this plugin's own source -
			-- the mismatch then surfaces as a confusing CLI argument error
			-- instead of something that points at the actual cause, so flag
			-- it here instead of staying quiet about which binary is in use.
			if vim.fn.executable("rot") == 1 then
				vim.notify(
					"[comment-rot] no local build at target/release/rot - using `"
						.. vim.fn.exepath("rot")
						.. "` from $PATH instead, which may not match this plugin's"
						.. " own source. Run `cargo build --release` (or `:Lazy build`) here to build a matching one.",
					vim.log.levels.WARN
				)
			end
		end
	end

	if vim.fn.executable(M.options.bin) == 0 then
		vim.notify(
			"[comment-rot] couldn't find the `rot` executable (looked for `"
				.. M.options.bin
				.. '`). Install it with `cargo install --path .`, or add `build = "cargo build --release"` to this plugin\'s lazy.nvim spec.',
			vim.log.levels.WARN
		)
	end
end

return M
