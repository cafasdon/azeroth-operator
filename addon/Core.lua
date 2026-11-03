-- Read-only exporter. No external command intake or gameplay actions.
local addonName, AO = ...
local function escape(value)
    return '"' .. value:gsub('[%z\1-\31\\"]', function(c)
        return string.format('\\u%04x', string.byte(c))
    end) .. '"'
end

local function json(value)
    local kind = type(value)
    if kind == "string" then return escape(value) end
    if kind == "number" then
        assert(value == value and value ~= math.huge and value ~= -math.huge, "Invalid number")
        return tostring(value)
    end
    if kind == "boolean" then return tostring(value) end
    if kind == "nil" then return "null" end
    assert(kind == "table", "Unsupported export value")
    local parts = {}
    -- Export objects only; slot maps avoid sparse-array ambiguity.
    for key, entry in pairs(value) do
        assert(type(key) == "string", "Export keys must be strings")
        parts[#parts + 1] = escape(key) .. ':' .. json(entry)
    end
    table.sort(parts)
    return '{' .. table.concat(parts, ',') .. '}'
end

function AO.Capture()
    assert(not InCombatLockdown(), "Export unavailable in combat")
    local version, build, _, interface = GetBuildInfo()
    assert(AO.family and AO.variant, "Client adapter missing")
    local sections = {}
    local equipment = {}
    for slot = 1, 19 do
        local id = GetInventoryItemID("player", slot)
        if id then
            equipment[tostring(slot)] = { item_id = id, link = GetInventoryItemLink("player", slot) }
        end
    end
    sections.equipment = equipment
    local bags = {}
    if C_Container and C_Container.GetContainerNumSlots and C_Container.GetContainerItemInfo then
        for bag = 0, (NUM_TOTAL_EQUIPPED_BAG_SLOTS or NUM_BAG_SLOTS or 4) do
            local slots = {}
            for slot = 1, C_Container.GetContainerNumSlots(bag) do
                local info = C_Container.GetContainerItemInfo(bag, slot)
                if info then slots[tostring(slot)] = { item_id = info.itemID, count = info.stackCount, link = info.hyperlink } end
            end
            bags[tostring(bag)] = slots
        end
        sections.bags = bags
    end
    sections.stats = { level = UnitLevel("player"), class = select(2, UnitClass("player")), race = select(2, UnitRace("player")), client_version = version }
    local snapshot = {
        schema_version = 1, source = "addon",
        captured_at = date("!%Y-%m-%dT%H:%M:%SZ", GetServerTime()),
        client = {family = AO.family, variant = AO.variant, build = tostring(build), interface = interface},
        character = {name = UnitName("player"), realm = GetRealmName()}, sections = sections
    }
    local payload = json(snapshot)
    assert(#payload <= 4 * 1024 * 1024, "Export too large")
    AzerothOperatorExport = payload:gsub('.', function(c) return string.format('%02x', string.byte(c)) end)
    return snapshot
end

SLASH_AZEROTHOPERATOR1 = "/aoexport"
SlashCmdList.AZEROTHOPERATOR = function()
    local ok, result = pcall(AO.Capture)
    if ok then
        print("Azeroth Operator: snapshot captured. Use /reload or log out to save it to disk.")
    else
        print("Azeroth Operator: export failed; previous snapshot retained. " .. tostring(result))
    end
end
