-- Read-only exporter. No external command intake or gameplay actions.
local addonName, AO = ...
local hexByte = {}
for byte = 0, 255 do
    hexByte[string.char(byte)] = string.format('%02x', byte)
end
local function escape(value)
    return '"' .. value:gsub('[%z\1-\31\\"]', function(c)
        return string.format('\\u%04x', string.byte(c))
    end) .. '"'
end

local function json(value, encoded)
    assert(not issecretvalue or not issecretvalue(value), "Restricted value")
    local kind = type(value)
    if kind == "string" then return escape(value) end
    if kind == "number" then
        assert(value == value and value ~= math.huge and value ~= -math.huge, "Invalid number")
        return tostring(value)
    end
    if kind == "boolean" then return tostring(value) end
    if kind == "nil" then return "null" end
    assert(kind == "table", "Unsupported export value")
    if encoded and encoded[value] then return encoded[value] end
    local parts = {}
    -- Export objects only; slot maps avoid sparse-array ambiguity.
    for key, entry in pairs(value) do
        assert(type(key) == "string", "Export keys must be strings")
        parts[#parts + 1] = escape(key) .. ':' .. json(entry, encoded)
    end
    table.sort(parts)
    return '{' .. table.concat(parts, ',') .. '}'
end

function AO.Capture(includeWallet)
    assert(not InCombatLockdown(), "Export unavailable in combat")
    local version, build, _, interface = GetBuildInfo()
    assert(AO.family and AO.variant, "Client adapter missing")
    assert(not AO.interface or AO.interface == interface, "Client interface changed; rebuild package")
    local sections = {}
    -- Reuse only successful section encodings within this synchronous capture.
    local encoded = {}
    local unavailable = {}
    local sectionNames={"equipment", "bags", "stats", "talents", "macros", "bindings", "action_bars", "professions", "quests", "currencies", "achievements", "operation_receipts", "binding_state", "action_bar_observation", "equipment_sets"}
    if AO.family=="forever" then
        sectionNames[#sectionNames+1]="character_bank"
        sectionNames[#sectionNames+1]="account_bank"
        sectionNames[#sectionNames+1]="selected_recipe"
        sectionNames[#sectionNames+1]="tracked_recipes"
        sectionNames[#sectionNames+1]="profession_recipe_view"
        sectionNames[#sectionNames+1]="mail_attachments"
        sectionNames[#sectionNames+1]="reputation"
        sectionNames[#sectionNames+1]="currency_balances"
        sectionNames[#sectionNames+1]="tracked_achievements"
        sectionNames[#sectionNames+1]="experience"
        sectionNames[#sectionNames+1]="spellbook"
        sectionNames[#sectionNames+1]="flight_map"
        sectionNames[#sectionNames+1]="durability_alerts"
        sectionNames[#sectionNames+1]="repair_quote"
        sectionNames[#sectionNames+1]="merchant_stock"
        sectionNames[#sectionNames+1]="trainer_services"
        sectionNames[#sectionNames+1]="pet_care"
        sectionNames[#sectionNames+1]="mount_collection"
        sectionNames[#sectionNames+1]="toy_collection_view"
        sectionNames[#sectionNames+1]="saved_instances"
        sectionNames[#sectionNames+1]="saved_world_bosses"
        sectionNames[#sectionNames+1]="skill_lines"
        sectionNames[#sectionNames+1]="bag_capacity"
        sectionNames[#sectionNames+1]="instance_context"
        if includeWallet then sectionNames[#sectionNames+1]="wallet" end
    end
    for _, name in ipairs(sectionNames) do
        local collector = AO.collectors and AO.collectors[name]
        if collector then
            local ok, data = pcall(collector, name=="pet_care" and sections.bags or nil)
            if ok and data then
                local serializable, text = pcall(json, data)
                if serializable then
                    sections[name] = data
                    encoded[data] = text
                else unavailable[name] = "restricted_or_unserializable" end
            else unavailable[name] = "api_unavailable_or_failed" end
        else unavailable[name] = "collector_not_implemented" end
    end
    local snapshot = {
        schema_version = 1, source = "addon",
        captured_at = date("!%Y-%m-%dT%H:%M:%SZ", GetServerTime()),
        client = {family = AO.family, variant = AO.variant, build = tostring(build), interface = interface},
        character = {name = UnitName("player"), realm = GetRealmName()}, sections = sections, unavailable = unavailable
    }
    local payload = json(snapshot, encoded)
    assert(#payload <= 4 * 1024 * 1024, "Export too large")
    AzerothOperatorExport = payload:gsub('.', hexByte)
    return snapshot
end

SLASH_AZEROTHOPERATOR1 = "/aoexport"
SlashCmdList.AZEROTHOPERATOR = function(message)
    local ok, result = pcall(AO.Capture, message == "wallet")
    if ok then
        print("Azeroth Operator: snapshot captured. Use /reload or log out to save it to disk.")
    else
        print("Azeroth Operator: export failed; previous snapshot retained. " .. tostring(result))
    end
end
