local config = require("comment-rot.config")
local job = require("comment-rot.job")
local theme = require("comment-rot.theme")
local util = require("comment-rot.util")

local M = {}

local function create_commands()
	vim.api.nvim_create_user_command("RotInit", function(cmd_opts)
		local root = util.find_root()
		local args = { "init", "--path", root }
		if cmd_opts.bang then
			table.insert(args, "--force")
		end

		util.notify(cmd_opts.bang and "rebuilding baseline..." or "creating baseline...")
		job.run(args, root, function(ok, _, stdout, stderr)
			if ok then
				util.notify(vim.trim(stdout))
			else
				util.notify("init failed: " .. job.err_message(stdout, stderr), vim.log.levels.ERROR)
			end
		end)
	end, {
		bang = true,
		desc = "Create a comment-rot baseline (! to force a from-scratch rebuild)",
	})

	vim.api.nvim_create_user_command("RotCheck", function()
		require("comment-rot.check").run()
	end, { desc = "Find stale comments and review them one at a time" })

	vim.api.nvim_create_user_command("RotStatus", function()
		require("comment-rot.status").run()
	end, { desc = "List comments confirmed stale but not yet fixed" })

	vim.api.nvim_create_user_command("RotResolve", function(cmd_opts)
		require("comment-rot.status").resolve(cmd_opts.args ~= "" and cmd_opts.args or nil)
	end, {
		nargs = "?",
		desc = "Resolve a known issue by id (or the entry under the cursor in :RotStatus)",
	})
end

function M.setup(opts)
	config.setup(opts)
	create_commands()

	theme.setup()
	vim.api.nvim_create_autocmd("ColorScheme", {
		group = vim.api.nvim_create_augroup("comment_rot_theme", { clear = true }),
		callback = theme.setup,
	})
end

return M
