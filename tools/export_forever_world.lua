-- Convert pinned QuestieDB's offline Generation rows to JSON lines. No game API.
local open, load, write = io.open, loadfile, io.write
local allowed = {}
for line in assert(open(arg[1], "r")):lines() do allowed[line] = true end

-- The publisher's loader may read only the staged, hash-checked source files.
-- Lua has no native socket API. Remove shell, native module and write access.
io = {open = function(path, mode)
  if not allowed[path] or (mode ~= nil and mode ~= "r" and mode ~= "rb") then return nil end
  return open(path, mode or "r")
end}
loadfile = function(path)
  if not allowed[path] then return nil, "Unrecognised source path" end
  return load(path)
end
dofile = function(path) return assert(loadfile(path))() end
package = {config = package.config}
os, require, module, debug = nil, nil, nil, nil
print = function() end

local config = dofile("src/config.lua")
local generation = dofile("generator/flavor.lua")
local loaded, stats = generation.load(config.flavorByName.Forever)

local function stringJSON(value)
  return '"' .. value:gsub('[%z\1-\31\\"]', function(c)
    if c == '"' or c == '\\' then return '\\' .. c end
    return string.format('\\u%04x', c:byte())
  end) .. '"'
end

local function json(value, depth)
  assert(depth <= 24, "World data nesting exceeds limit")
  local kind = type(value)
  if kind == "string" then return stringJSON(value) end
  if kind == "boolean" then return tostring(value) end
  if kind == "number" then
    assert(value == value and value > -math.huge and value < math.huge, "Non-finite world number")
    return string.format("%.17g", value)
  end
  assert(kind == "table", "Unsupported world field type")
  local keys, seen, parts = {}, {}, {}
  for key in pairs(value) do
    assert(type(key) == "string" or (type(key) == "number" and key % 1 == 0), "Invalid world field key")
    local text = type(key) == "number" and string.format("%.0f", key) or key
    assert(not seen[text], "Duplicate JSON key")
    keys[#keys+1], seen[text] = text, key
  end
  table.sort(keys)
  for _, key in ipairs(keys) do
    parts[#parts+1] = stringJSON(key) .. ':' .. json(value[seen[key]], depth+1)
  end
  return '{' .. table.concat(parts, ',') .. '}'
end

local bytes, count = 0, 0
local function emit(value)
  local line = json(value, 0)
  bytes, count = bytes + #line + 1, count + 1
  assert(bytes <= 64*1024*1024 and count <= 100001, "World export exceeds bound")
  assert(write(line, '\n'))
end
emit({contract=config.contractVersion, static_corrections=stats.applied, derived_passes=stats.derived})
for _, entityType in ipairs(config.entityTypes) do
  local entry = assert(loaded[entityType.name])
  local ids = {}
  for id in pairs(entry.entities) do ids[#ids+1] = id end
  table.sort(ids)
  for _, id in ipairs(ids) do
    local fields = {}
    for index, value in pairs(entry.entities[id]) do
      local name = assert(entry.meta.names[index], "Unrecognised world field")
      fields[name] = value
    end
    emit({kind=entry.meta.entity:lower(), id=id, fields=fields})
  end
end
