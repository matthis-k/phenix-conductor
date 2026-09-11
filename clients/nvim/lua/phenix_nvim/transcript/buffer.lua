local M = {}
local namespace = vim.api.nvim_create_namespace("phenix-transcript")
local buffer
local marks = {}

local function lines_for(node)
  if node.kind == "message" then
    if node.role == "user" then
      return { "## You", "", node.text, "" }
    end
    return { node.text, "" }
  end
  if node.kind == "tool" then
    local output = node.output and vim.inspect(node.output) or ""
    return {
      "### Tool · " .. tostring(node.callable_id),
      "",
      "`" .. node.state .. "`",
      output,
      "",
    }
  end
  if node.kind == "progress" then
    return { "_" .. (node.message or "working") .. "_", "" }
  end
  return { vim.inspect(node), "" }
end

function M.ensure()
  if buffer ~= nil and vim.api.nvim_buf_is_valid(buffer) then
    return buffer
  end
  buffer = vim.api.nvim_create_buf(false, true)
  vim.bo[buffer].buftype = "nofile"
  vim.bo[buffer].bufhidden = "hide"
  vim.bo[buffer].modifiable = false
  vim.bo[buffer].swapfile = false
  vim.bo[buffer].filetype = "markdown"
  vim.api.nvim_buf_set_name(buffer, "phenix://transcript")
  marks = {}
  return buffer
end

local function replace(start_row, finish_row, lines)
  local target = M.ensure()
  vim.bo[target].modifiable = true
  vim.api.nvim_buf_set_lines(target, start_row, finish_row, false, lines)
  vim.bo[target].modifiable = false
end

function M.render_node(node)
  local target = M.ensure()
  local existing = marks[node.id]
  local lines = lines_for(node)
  local start_row
  if existing ~= nil then
    local position = vim.api.nvim_buf_get_extmark_by_id(target, namespace, existing, { details = true })
    if #position == 0 then
      marks[node.id] = nil
      return M.render_node(node)
    end
    start_row = position[1]
    local finish_row = (position[3].end_row or position[1]) + 1
    replace(start_row, finish_row, lines)
  else
    start_row = vim.api.nvim_buf_line_count(target)
    if start_row == 1 and vim.api.nvim_buf_get_lines(target, 0, 1, false)[1] == "" then
      start_row = 0
      replace(0, 1, lines)
    else
      replace(start_row, start_row, lines)
    end
  end
  local end_row = start_row + math.max(#lines - 1, 0)
  marks[node.id] = vim.api.nvim_buf_set_extmark(target, namespace, start_row, 0, {
    id = existing,
    end_row = end_row,
    end_col = #(lines[#lines] or ""),
    right_gravity = false,
  })
end

function M.render_projection(projection)
  local target = M.ensure()
  vim.bo[target].modifiable = true
  vim.api.nvim_buf_set_lines(target, 0, -1, false, {})
  vim.api.nvim_buf_clear_namespace(target, namespace, 0, -1)
  vim.bo[target].modifiable = false
  marks = {}
  for _, id in ipairs(projection.order) do
    M.render_node(projection.nodes[id])
  end
end

return M
