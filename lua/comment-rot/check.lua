local job = require("comment-rot.job")
local util = require("comment-rot.util")

local M = {}

local ns = vim.api.nvim_create_namespace("comment_rot_check")

local MARGIN = 2 -- left padding inside the content window
local FOOTER_HEIGHT = 1

-- One "line" is built up as colored chunks rather than a single hl per
-- line, so e.g. the header can mix a kind badge, the item path, and the
-- progress dots in one row without fighting over one highlight group.
local function chunked_line(chunks)
	local text = ""
	local segs = {}
	for _, c in ipairs(chunks) do
		local s = type(c) == "table" and c[1] or c
		local hl = type(c) == "table" and c[2] or nil
		local start_col = #text
		text = text .. s
		if hl then
			segs[#segs + 1] = { start_col, #text, hl }
		end
	end
	return { text = text, segs = segs }
end

local function progress_chunks(index, total)
	if total > 24 then
		return { { string.format("%d / %d", index, total), "CommentRotMuted" } }
	end
	local chunks = {}
	for i = 1, total do
		if i > 1 then
			chunks[#chunks + 1] = { " ", nil }
		end
		if i < index then
			chunks[#chunks + 1] = { "●", "CommentRotProgressDone" }
		elseif i == index then
			chunks[#chunks + 1] = { "◐", "CommentRotAccent" }
		else
			chunks[#chunks + 1] = { "○", "CommentRotProgressPending" }
		end
	end
	return chunks
end

local function rule(width)
	return chunked_line({ { string.rep("─", math.max(width, 1)), "CommentRotBorder" } })
end

local function keymap_hint_line(width)
	local chunks = {
		{ "y", "CommentRotKey" },
		{ " accurate    ", "CommentRotMuted" },
		{ "n", "CommentRotKey" },
		{ " stale    ", "CommentRotMuted" },
		{ "g", "CommentRotKey" },
		{ " jump & fix    ", "CommentRotMuted" },
		{ "q", "CommentRotKey" },
		{ " quit", "CommentRotMuted" },
	}
	local len = 0
	for _, c in ipairs(chunks) do
		len = len + #c[1]
	end
	local pad = math.max(0, math.floor((width - len) / 2))
	table.insert(chunks, 1, { string.rep(" ", pad), nil })
	return chunked_line(chunks)
end

--- Builds every line of the review buffer for one candidate as a list of
--- `{text, segs, sign}` rows, right-aligning the header's progress dots
--- against `width` (the content window's fixed inner width).
local function build(candidate, index, total, width)
	local rows = {}

	local function push(row)
		rows[#rows + 1] = row
		return row
	end

	push(chunked_line({
		{ util.kind_label(candidate.kind):upper(), util.kind_hl(candidate.kind) },
		{ "  ", nil },
		{ candidate.item_path, "CommentRotAccent" },
	}))
	push(chunked_line({ { candidate.file .. ":" .. candidate.line, "CommentRotMuted" } }))
	push(rule(width))
	push(chunked_line({ { "Comment", "CommentRotMuted" } }))
	for _, l in ipairs(vim.split(candidate.comment_text, "\n", { trimempty = true })) do
		push(chunked_line({ { "▏ ", "CommentRotQuote" }, { l, "CommentRotQuote" } }))
	end
	push(chunked_line({}))
	push(chunked_line({ { "Diff", "CommentRotMuted" } }))

	local unified = vim.text.diff(
		candidate.old_body_text,
		candidate.new_body_text,
		{ result_type = "unified", ctxlen = 3 }
	) or ""
	for _, l in ipairs(vim.split(unified, "\n", { trimempty = true })) do
		local c = l:sub(1, 1)
		if c == "\\" then
			-- "\ No newline at end of file" - body_text is sliced straight out
			-- of the source and routinely lacks one; not worth showing.
		elseif c == "@" then
			push(chunked_line({ { "⋯⋯⋯", "CommentRotHunk" } }))
		elseif c == "+" then
			local row = push(chunked_line({ { "+ " .. l:sub(2), "CommentRotDiffAdd" } }))
			row.sign = { text = "+ ", hl = "CommentRotDiffAdd" }
		elseif c == "-" then
			local row = push(chunked_line({ { "- " .. l:sub(2), "CommentRotDiffDelete" } }))
			row.sign = { text = "- ", hl = "CommentRotDiffDelete" }
		else
			push(chunked_line({ { "  " .. l:sub(2), "CommentRotDiffContext" } }))
		end
	end

	push(chunked_line({}))
	push(chunked_line({ { candidate.reason, "CommentRotReason" } }))

	local progress = progress_chunks(index, total)
	local plen = 0
	for _, c in ipairs(progress) do
		plen = plen + #c[1]
	end
	local header = rows[1]
	local pad = math.max(1, width - vim.fn.strdisplaywidth(header.text) - plen)
	header.text = header.text .. string.rep(" ", pad)
	local cursor = #header.text
	for _, c in ipairs(progress) do
		local start_col = cursor
		header.text = header.text .. c[1]
		cursor = cursor + #c[1]
		if c[2] then
			header.segs[#header.segs + 1] = { start_col, cursor, c[2] }
		end
	end

	return rows
end

local function fixed_sizes()
	local width = math.min(math.max(math.floor(vim.o.columns * 0.8), 60), 100)
	local content_height = math.min(math.max(math.floor(vim.o.lines * 0.7), 15), 40)
	return width, content_height
end

--- Two floating windows, not one: a scrollable content window (comment +
--- diff, which can run long) and a separate, fixed-height, unfocusable
--- footer pinned directly beneath it. Splitting them out is what keeps
--- the keymap hints visible no matter how big a given candidate's diff
--- is - cramming both into one auto-sized window meant a large diff could
--- push the hints below the fold with no visible way back to them.
local function open_windows()
	local width, content_height = fixed_sizes()
	local combined_height = content_height + FOOTER_HEIGHT + 4 -- both windows' own borders
	local row = math.floor((vim.o.lines - combined_height) / 2)
	local col = math.floor((vim.o.columns - width) / 2)

	local content_buf = vim.api.nvim_create_buf(false, true)
	vim.bo[content_buf].bufhidden = "wipe"
	vim.bo[content_buf].buftype = "nofile"
	vim.bo[content_buf].swapfile = false
	vim.bo[content_buf].filetype = "commentrot"

	local content_win = vim.api.nvim_open_win(content_buf, true, {
		relative = "editor",
		width = width,
		height = content_height,
		row = row,
		col = col,
		style = "minimal",
		border = "rounded",
		title = " ✎ comment-rot ",
		title_pos = "center",
	})
	vim.wo[content_win].wrap = false
	vim.wo[content_win].cursorline = false
	vim.wo[content_win].signcolumn = "yes:1"
	vim.wo[content_win].winhighlight = "FloatBorder:CommentRotBorder,FloatTitle:CommentRotAccent"

	local footer_buf = vim.api.nvim_create_buf(false, true)
	vim.bo[footer_buf].bufhidden = "wipe"
	vim.bo[footer_buf].buftype = "nofile"
	vim.bo[footer_buf].swapfile = false

	local footer_win = vim.api.nvim_open_win(footer_buf, false, {
		relative = "editor",
		width = width,
		height = FOOTER_HEIGHT,
		row = row + content_height + 2,
		col = col,
		style = "minimal",
		border = "rounded",
		focusable = false,
	})
	vim.wo[footer_win].winhighlight = "FloatBorder:CommentRotBorder"

	-- The footer's text never changes between candidates, so fill it once
	-- rather than on every render().
	local hint = keymap_hint_line(width)
	vim.bo[footer_buf].modifiable = true
	vim.api.nvim_buf_set_lines(footer_buf, 0, -1, false, { hint.text })
	vim.bo[footer_buf].modifiable = false
	for _, seg in ipairs(hint.segs) do
		vim.api.nvim_buf_set_extmark(footer_buf, ns, 0, seg[1], { end_col = seg[2], hl_group = seg[3] })
	end

	-- Safety net: if the content window ever closes through some path
	-- other than our own `close()` (e.g. `<C-w>c`), don't leave the
	-- unfocusable footer window stranded on screen.
	vim.api.nvim_create_autocmd("WinClosed", {
		pattern = tostring(content_win),
		once = true,
		callback = function()
			if vim.api.nvim_win_is_valid(footer_win) then
				vim.api.nvim_win_close(footer_win, true)
			end
		end,
	})

	return content_buf, content_win, footer_win, width
end

local function render(content_buf, content_win, width, candidate, index, total)
	vim.api.nvim_buf_clear_namespace(content_buf, ns, 0, -1)

	local rows = build(candidate, index, total, width - MARGIN * 2)

	vim.bo[content_buf].modifiable = true
	local lines = {}
	for i, row in ipairs(rows) do
		lines[i] = string.rep(" ", MARGIN) .. row.text
	end
	vim.api.nvim_buf_set_lines(content_buf, 0, -1, false, lines)
	vim.bo[content_buf].modifiable = false

	for i, row in ipairs(rows) do
		for _, seg in ipairs(row.segs) do
			vim.api.nvim_buf_set_extmark(
				content_buf,
				ns,
				i - 1,
				seg[1] + MARGIN,
				{ end_col = seg[2] + MARGIN, hl_group = seg[3] }
			)
		end
		if row.sign then
			vim.api.nvim_buf_set_extmark(
				content_buf,
				ns,
				i - 1,
				0,
				{ sign_text = row.sign.text, sign_hl_group = row.sign.hl }
			)
		end
	end

	-- Reset scroll position to the top for the new candidate; if its
	-- content is taller than the window, normal scrolling (`<C-d>`, `j`,
	-- `G`, ...) works exactly as it would in any other buffer.
	vim.api.nvim_win_set_cursor(content_win, { 1, 0 })
end

--- Opens the candidate's file at its line, closing the review float first.
--- Deliberately doesn't try to resume the walkthrough afterwards: editing
--- the comment to match the already-changed code makes both differ from
--- the baseline, which the CLI already treats as an intentional pairing
--- and clears silently on the next `:RotCheck` - no verdict needed here.
local function jump_to_code(root, candidate)
	local path = vim.fs.joinpath(root, candidate.file)
	if not pcall(vim.cmd.edit, path) then
		vim.cmd.split(path)
	end
	vim.api.nvim_win_set_cursor(0, { candidate.line, 0 })
	vim.cmd("normal! zz")
end

--- Drives the float through `candidates` one at a time, collecting a
--- `{id, verdict}` per y/n keypress. `on_finished(verdicts, jump)` fires
--- once the list is exhausted, the user quits early, or jumps to fix code
--- directly - verdicts already given are kept in every case, `jump` is the
--- candidate to open if the user pressed `g`.
local function walkthrough(root, candidates, on_finished)
	local content_buf, content_win, footer_win, width = open_windows()
	local verdicts = {}
	local idx = 1

	local function close()
		if vim.api.nvim_win_is_valid(content_win) then
			vim.api.nvim_win_close(content_win, true)
		end
		if vim.api.nvim_win_is_valid(footer_win) then
			vim.api.nvim_win_close(footer_win, true)
		end
	end

	local function step()
		if idx > #candidates then
			close()
			on_finished(verdicts, nil)
			return
		end
		render(content_buf, content_win, width, candidates[idx], idx, #candidates)
	end

	local function record(verdict)
		verdicts[#verdicts + 1] = { id = candidates[idx].id, verdict = verdict }
		idx = idx + 1
		step()
	end

	local opts = { buffer = content_buf, nowait = true, silent = true }
	vim.keymap.set("n", "y", function()
		record("yes")
	end, opts)
	vim.keymap.set("n", "n", function()
		record("no")
	end, opts)
	vim.keymap.set("n", "g", function()
		local candidate = candidates[idx]
		close()
		on_finished(verdicts, candidate)
	end, opts)
	local function quit()
		close()
		on_finished(verdicts, nil)
	end
	vim.keymap.set("n", "q", quit, opts)
	vim.keymap.set("n", "<Esc>", quit, opts)

	step()
end

function M.run()
	local root = util.find_root()
	util.notify("checking for stale comments...")

	job.run({ "check", "--path", root, "--json" }, root, function(ok, decoded, stdout, stderr)
		if not ok or not decoded then
			util.notify("check failed: " .. job.err_message(stdout, stderr), vim.log.levels.ERROR)
			return
		end

		local candidates = decoded.candidates or {}
		if #candidates == 0 then
			util.notify(
				string.format(
					"no stale comments found (%d added, %d updated, %d unchanged, %d pruned)",
					decoded.added,
					decoded.updated,
					decoded.unchanged,
					decoded.pruned
				)
			)
			return
		end

		walkthrough(root, candidates, function(verdicts, jump)
			local function maybe_jump()
				if jump then
					jump_to_code(root, jump)
				end
			end

			if #verdicts == 0 then
				if jump then
					maybe_jump()
				end
				return
			end

			job.run({ "confirm", "--path", root }, root, function(ok2, decoded2, stdout2, stderr2)
				if not ok2 or not decoded2 then
					util.notify("confirm failed: " .. job.err_message(stdout2, stderr2), vim.log.levels.ERROR)
					return
				end
				util.notify(
					string.format(
						"%d baseline entries updated, %d flagged as known issue(s)",
						decoded2.updated,
						decoded2.flagged
					)
				)
				maybe_jump()
			end, vim.json.encode(verdicts))
		end)
	end)
end

return M
