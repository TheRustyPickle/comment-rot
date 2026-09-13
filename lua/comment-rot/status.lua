local job = require("comment-rot.job")
local util = require("comment-rot.util")

local M = {}

local QF_TITLE = "comment-rot: known issues"

--- id -> absolute root, so actions taken from the quickfix window (which
--- only carries the id embedded in its `text` field) know which project
--- the listed issue belongs to.
local roots_by_id = {}

local function to_qf_item(root, entry)
	local issue = entry.issue
	roots_by_id[entry.id] = root
	local abs = vim.fs.joinpath(root, issue.file)
	return {
		filename = abs,
		lnum = util.locate_comment_line(abs, issue.comment_text, 1),
		col = 1,
		text = string.format("[%s] %s `%s` - %s", entry.id, util.kind_label(issue.kind), issue.item_path, issue.reason),
	}
end

--- True only in the exact quickfix window/list `:RotStatus` itself
--- populated - other quickfix lists (e.g. from `:grep`) aren't ours.
local function in_status_list()
	return vim.bo.buftype == "quickfix" and vim.fn.getqflist({ title = 0 }).title == QF_TITLE
end

--- Closes our own quickfix window if it's open, wherever it's docked -
--- used once the list empties out (the last issue got resolved/re-verdicted
--- from inside it) so a stale, now-empty list doesn't just sit there.
--- Neovim has one global quickfix list shown by every quickfix-type window
--- (location-list windows are the separate, per-window kind), so checking
--- the current list's title once is enough before closing all of them.
local function close_if_open()
	if vim.fn.getqflist({ title = 0 }).title ~= QF_TITLE then
		return
	end
	for _, win in ipairs(vim.fn.getwininfo()) do
		if win.quickfix == 1 and win.loclist == 0 then
			pcall(vim.api.nvim_win_close, win.winid, true)
		end
	end
end

local function id_under_cursor()
	local qf_id = vim.fn.getqflist({ id = 0 }).id
	local idx = vim.fn.line(".")
	local list = vim.fn.getqflist({ id = qf_id, items = 0 }).items
	local current = list[idx]
	if not current then
		return nil, nil
	end
	return current.text:match("^%[(.-)%]"), idx
end

local function do_resolve(root, id, on_done)
	job.run({ "resolve", "--path", root, id, "--json" }, root, function(ok, decoded, stdout, stderr)
		if decoded and decoded.resolved then
			util.notify("resolved " .. id)
		elseif decoded then
			util.notify("no known issue with id " .. id, vim.log.levels.WARN)
		else
			util.notify("resolve failed: " .. job.err_message(stdout, stderr), vim.log.levels.ERROR)
		end
		if on_done then
			on_done(ok)
		end
	end)
end

local function do_verdict(root, id, verdict, on_done)
	job.run({ "confirm", "--path", root }, root, function(ok, decoded, stdout, stderr)
		if not ok or not decoded then
			util.notify("confirm failed: " .. job.err_message(stdout, stderr), vim.log.levels.ERROR)
			if on_done then
				on_done(false)
			end
			return
		end
		util.notify(verdict == "yes" and ("marked accurate: " .. id) or ("re-flagged as stale: " .. id))
		if on_done then
			on_done(true)
		end
	end, vim.json.encode({ { id = id, verdict = verdict } }))
end

--- Binds y/n/r directly in the quickfix buffer `:RotStatus` just opened,
--- so acting on an entry doesn't require first working out its id: `y`
--- re-confirms the comment as accurate, `n` re-flags it (refreshing the
--- stale body it's compared against), `r` dismisses it outright without
--- touching the baseline. All three refresh the list afterward, keeping
--- the cursor roughly where it was.
local function setup_qf_keymaps(buf)
	local function act(handler)
		local id, idx = id_under_cursor()
		if not id then
			util.notify("no known-issue entry under the cursor", vim.log.levels.ERROR)
			return
		end
		local root = roots_by_id[id] or util.find_root()
		handler(root, id, function(ok)
			if ok then
				M.run(idx)
			end
		end)
	end

	local opts = { buffer = buf, nowait = true, silent = true }
	vim.keymap.set("n", "y", function()
		act(function(root, id, cb)
			do_verdict(root, id, "yes", cb)
		end)
	end, opts)
	vim.keymap.set("n", "n", function()
		act(function(root, id, cb)
			do_verdict(root, id, "no", cb)
		end)
	end, opts)
	vim.keymap.set("n", "r", function()
		act(do_resolve)
	end, opts)
end

--- Lists known issues in the quickfix window. `restore_idx`, if given, is
--- the line to put the cursor back on after an action refreshes the list.
function M.run(restore_idx)
	local root = util.find_root()

	job.run({ "status", "--path", root, "--json" }, root, function(ok, decoded, stdout, stderr)
		if not ok or not decoded then
			util.notify("status failed: " .. job.err_message(stdout, stderr), vim.log.levels.ERROR)
			return
		end

		if #decoded == 0 then
			util.notify("no known issues")
			-- Regression: resolving/re-verdicting the last remaining entry
			-- from inside the list used to leave that now-empty, stale list
			-- open instead of closing it.
			vim.fn.setqflist({}, "r", { title = QF_TITLE, items = {} })
			close_if_open()
			return
		end

		local items = {}
		for _, entry in ipairs(decoded) do
			items[#items + 1] = to_qf_item(root, entry)
		end

		vim.fn.setqflist({}, " ", {
			title = QF_TITLE,
			items = items,
		})
		vim.cmd.copen()
		vim.wo.list = false
		-- A one-shot notify is easy to miss (scrolls away, gets buried by a
		-- noisy notifier plugin) and the keymaps only mean anything while
		-- this window is open, so pin the hint to its winbar instead -
		-- visible the whole time, gone once the window is.
		vim.wo.winbar = "%#CommentRotKey#y%#CommentRotMuted# accurate    "
			.. "%#CommentRotKey#n%#CommentRotMuted# stale    "
			.. "%#CommentRotKey#r%#CommentRotMuted# dismiss    "
			.. "%#CommentRotKey#<CR>%#CommentRotMuted# jump"
		setup_qf_keymaps(vim.api.nvim_get_current_buf())

		if restore_idx then
			vim.fn.cursor(math.min(restore_idx, #items), 1)
		end
	end)
end

--- Resolves `id`. With no argument: uses the entry under the cursor if
--- called from the list `:RotStatus` opened, otherwise (e.g. from a
--- keymap bound outside that window) prompts with `vim.ui.select` so a
--- plain keymap has something to act on.
function M.resolve(id)
	if id then
		do_resolve(roots_by_id[id] or util.find_root(), id)
		return
	end

	if in_status_list() then
		local cursor_id, idx = id_under_cursor()
		if not cursor_id then
			util.notify("no known-issue entry under the cursor", vim.log.levels.ERROR)
			return
		end
		do_resolve(roots_by_id[cursor_id] or util.find_root(), cursor_id, function(ok)
			if ok then
				M.run(idx)
			end
		end)
		return
	end

	local root = util.find_root()
	job.run({ "status", "--path", root, "--json" }, root, function(ok, decoded, stdout, stderr)
		if not ok or not decoded then
			util.notify("status failed: " .. job.err_message(stdout, stderr), vim.log.levels.ERROR)
			return
		end
		if #decoded == 0 then
			util.notify("no known issues")
			return
		end

		vim.ui.select(decoded, {
			prompt = "Resolve which known issue?",
			format_item = function(entry)
				local issue = entry.issue
				return string.format(
					"%s `%s` - %s (%s)",
					util.kind_label(issue.kind),
					issue.item_path,
					issue.reason,
					issue.file
				)
			end,
		}, function(choice)
			if choice then
				do_resolve(root, choice.id)
			end
		end)
	end)
end

return M
