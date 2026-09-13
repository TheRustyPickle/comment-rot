local config = require("comment-rot.config")

local M = {}

--- Runs `rot <args>` in `cwd` and calls `on_done(ok, decoded, stdout, stderr, code)`.
--- `decoded` is the parsed JSON body of stdout, or nil if stdout wasn't
--- valid JSON (e.g. the CLI errored before ever printing its payload).
--- `stdin`, if given, is written to the process's stdin and closed.
function M.run(args, cwd, on_done, stdin)
	local cmd = { config.options.bin }
	vim.list_extend(cmd, args)

	local opts = { cwd = cwd, text = true }
	if stdin then
		opts.stdin = stdin
	end

	vim.system(cmd, opts, function(res)
		vim.schedule(function()
			local ok = res.code == 0
			local decoded = nil
			if res.stdout and res.stdout ~= "" then
				local success, parsed = pcall(vim.json.decode, res.stdout)
				if success then
					decoded = parsed
				end
			end
			on_done(ok, decoded, res.stdout or "", res.stderr or "", res.code)
		end)
	end)
end

--- The message worth surfacing to the user when a command failed: stderr
--- if the CLI wrote one, otherwise whatever landed on stdout.
function M.err_message(stdout, stderr)
	if stderr ~= "" then
		return vim.trim(stderr)
	end
	return vim.trim(stdout)
end

return M
