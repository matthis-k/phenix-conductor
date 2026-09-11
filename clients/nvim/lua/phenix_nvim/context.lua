local M = {}

local function uri(buffer)
  local name = vim.api.nvim_buf_get_name(buffer)
  if name == "" then
    return nil
  end
  return vim.uri_from_fname(vim.fn.fnamemodify(name, ":p"))
end

function M.current_location()
  local buffer = vim.api.nvim_get_current_buf()
  local cursor = vim.api.nvim_win_get_cursor(0)
  return {
    kind = "location",
    source = {
      uri = uri(buffer),
      line = cursor[1] - 1,
      column = cursor[2],
    },
  }
end

function M.visual_selection()
  local mode = vim.fn.mode(1)
  if mode == "\22" then
    return nil, "blockwise references are not supported yet"
  end
  if mode ~= "v" and mode ~= "V" then
    return nil, "Reference requires an active visual selection"
  end

  local buffer = vim.api.nvim_get_current_buf()
  local start = vim.fn.getpos("v")
  local finish = vim.fn.getpos(".")
  if start[2] > finish[2] or (start[2] == finish[2] and start[3] > finish[3]) then
    start, finish = finish, start
  end

  local start_row = start[2] - 1
  local end_row = finish[2] - 1
  local start_col = mode == "V" and 0 or start[3] - 1
  local end_col
  local lines
  if mode == "V" then
    lines = vim.api.nvim_buf_get_lines(buffer, start_row, end_row + 1, false)
    end_col = #(lines[#lines] or "")
  else
    end_col = finish[3]
    lines = vim.api.nvim_buf_get_text(buffer, start_row, start_col, end_row, end_col, {})
  end

  return {
    kind = "selection",
    source = {
      uri = uri(buffer),
      start_line = start_row,
      start_column = start_col,
      end_line = end_row,
      end_column = end_col,
    },
    snapshot = table.concat(lines, "\n"),
  }
end

return M
