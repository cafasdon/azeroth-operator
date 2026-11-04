local _, AO = ...
local panel
local function reason(value)
    if type(value)~="string" then return "No additional detail." end
    -- Render diagnostics as bounded text, never WoW links or colour escapes.
    return value:gsub("[%c]"," "):gsub("|","/"):sub(1,240)
end
local function open()
    if panel then panel:Show(); return end
    panel=CreateFrame("Frame", "AzerothOperatorMacroPanel", UIParent, "BasicFrameTemplateWithInset")
    panel:SetSize(620,540)
    panel:SetPoint("CENTER")
    panel:SetMovable(true)
    panel:EnableMouse(true)
    panel:RegisterForDrag("LeftButton")
    panel:SetScript("OnDragStart",panel.StartMoving)
    panel:SetScript("OnDragStop",panel.StopMovingOrSizing)
    panel.TitleText:SetText("Azeroth Operator — review operation")
    local function textbox(y,height)
        local scroll=CreateFrame("ScrollFrame",nil,panel,"UIPanelScrollFrameTemplate")
        scroll:SetPoint("TOPLEFT",18,y)
        scroll:SetSize(558,height)
        local box=CreateFrame("EditBox",nil,scroll)
        box:SetMultiLine(true)
        box:SetFontObject(ChatFontNormal)
        box:SetWidth(550)
        box:SetHeight(height)
        box:EnableMouse(true)
        box:SetAutoFocus(false)
        box:SetScript("OnEscapePressed",box.ClearFocus)
        scroll:SetScrollChild(box)
        local background=scroll:CreateTexture(nil,"BACKGROUND")
        background:SetAllPoints()
        background:SetColorTexture(0.05,0.05,0.05,0.9)
        return box
    end
    local input=textbox(-42,105)
    input:SetMaxLetters(524288)
    local hint=panel:CreateFontString(nil,"OVERLAY","GameFontHighlightSmall")
    hint:SetPoint("TOPLEFT",18,-28)
    hint:SetText("Click the dark box below and paste the operation code (Ctrl+V).")
    input:SetScript("OnMouseDown",function(self) self:SetFocus() end)
    local details=textbox(-195,265)
    local review=CreateFrame("Button",nil,panel,"UIPanelButtonTemplate")
    review:SetSize(150,26); review:SetPoint("TOPLEFT",18,-155); review:SetText("Review pasted code")
    local apply=CreateFrame("Button",nil,panel,"UIPanelButtonTemplate")
    apply:SetSize(150,26); apply:SetPoint("BOTTOMRIGHT",-18,15); apply:SetText("Apply this operation")
    apply:Disable()
    local status=panel:CreateFontString(nil,"OVERLAY","GameFontHighlightSmall")
    status:SetPoint("BOTTOMLEFT",18,23); status:SetWidth(420); status:SetJustifyH("LEFT")
    status:SetText("Paste an operation code, then review it.")
    input:SetScript("OnTextChanged",function()
        AO.InvalidateMacroReview(); apply:Disable(); details:SetText("")
        status:SetText("Review the current code before applying.")
    end)
    review:SetScript("OnClick",function()
        apply:Disable()
        local ok,op=pcall(AO.ReviewMacroImport,input:GetText())
        if not ok then status:SetText("Invalid operation code."); return end
        local lines={"Operation: "..op.kind,"Character: "..op.character.." — "..op.realm,
            "Client: "..op.family.." / "..op.variant.." / "..op.build,
            "From: "..(op.source_scope or "none"),"To: "..(op.target_scope or op.source_scope or "none")}
        if op.kind=="create_equipment_set" or op.kind=="save_equipment_set" then
            lines[#lines+1]="Equipment set: "..reason(op.name)..(op.set_id~=nil and " (ID "..op.set_id..")" or " (new)")
            lines[#lines+1]="Saved icon: "..op.icon
            local old=op.set_id~=nil and op.expected_catalogue[tostring(op.set_id)] or nil
            lines[#lines+1]=old and "OVERWRITE this saved set's item contents with the reviewed equipped items." or "Create a new set from the reviewed equipped items."
            lines[#lines+1]="Preserved specialisation index: "..(old and old.assigned_spec or "None")
            for slot=1,19 do
                local key=tostring(slot); local item=op.equipment_before[key]
                local previous=old and old.slots[key]
                local before=previous and (previous.ignored==true and "Ignored" or previous.observed_item and reason(previous.observed_item.guid) or "Unresolved saved identity") or "New set"
                lines[#lines+1]="Slot "..slot.." / Previous: "..before
                lines[#lines+1]="  "..(op.ignored_slots[key] and "Ignore in saved set" or "Save item "..item.item_id).." / Equipped: "..reason(item.guid~="" and item.guid or "Empty (checked on Apply)")
            end
            lines[#lines+1]="Close the native equipment-set editor and confirmation dialogs before Apply."
            lines[#lines+1]="Current equipment and all captured sets are checked again. Gear is not equipped by this operation."
            lines[#lines+1]="After completion, /aoexport within 60 seconds, then /reload. Do not repeat an uncertain operation."
        elseif op.kind=="assign_equipment_set_spec" or op.kind=="unassign_equipment_set_spec" then
            local target=op.expected_catalogue[tostring(op.set_id)]
            lines[#lines+1]="Equipment set: "..reason(target.name).." (ID "..op.set_id..")"
            lines[#lines+1]="Previous specialisation index: "..(target.assigned_spec or "None")
            if op.spec_index then
                lines[#lines+1]="Assign to specialisation index "..op.spec_index.." / ID "..op.spec_id
                if op.displaced_set_id~=nil then
                    local displaced=op.expected_catalogue[tostring(op.displaced_set_id)]
                    lines[#lines+1]="Remove this association from: "..reason(displaced.name).." (ID "..op.displaced_set_id..")"
                else lines[#lines+1]="No other set association is displaced." end
            else lines[#lines+1]="Remove this set's specialisation association." end
            lines[#lines+1]="Future specialisation changes may select different equipment."
            lines[#lines+1]="No specialisation switch or equipment-use call is requested now."
            lines[#lines+1]="All captured sets and specialisation identities are checked again."
            lines[#lines+1]="After completion, /aoexport within 60 seconds, then /reload. Do not repeat an uncertain operation."
        elseif op.kind=="modify_equipment_set" or op.kind=="delete_equipment_set" or op.kind=="use_equipment_set" then
            lines[#lines+1]="Equipment set: "..reason(op.expected.name).." (ID "..op.set_id..")"
            lines[#lines+1]="Current icon: "..op.expected.icon
            lines[#lines+1]="Assigned specialisation index: "..(op.expected.assigned_spec or "None")
            lines[#lines+1]="Saved items: "..op.expected.num_items.." / Missing: "..op.expected.num_lost.." / Ignored slots: "..op.expected.num_ignored
            if op.kind=="modify_equipment_set" then
                lines[#lines+1]="New name: "..reason(op.name)
                lines[#lines+1]="New icon: "..op.icon
                lines[#lines+1]="Changes the saved set name/icon only; no gear is equipped or saved."
            elseif op.kind=="use_equipment_set" then
                lines[#lines+1]="Equip this saved set. Ignored slots must remain unchanged."
                for slot=1,19 do
                    local key=tostring(slot)
                    local target=op.expected.slots[key]
                    local before=op.equipment_before[key]
                    local desired=target.ignored and before or target.observed_item.guid
                    local label=target.ignored and "Preserve" or "Equip"
                    lines[#lines+1]="Slot "..slot.." / "..label..": "..reason(before~="" and before or "Empty (checked on Apply)").." -> "..reason(desired~="" and desired or "Empty")
                end
                lines[#lines+1]="Completion requires all equipped item identities to match. A partial swap remains unconfirmed."
            else
                lines[#lines+1]="DELETE this saved equipment set and its specialisation association."
                lines[#lines+1]="This does not delete the items. References to the set may stop working."
            end
            lines[#lines+1]="Exact captured set contents are checked again. External references are not repaired."
            lines[#lines+1]="After completion, /aoexport within 60 seconds, then /reload. Do not repeat an uncertain operation."
        elseif op.kind=="create_talent_config" then
            lines[#lines+1]="Create saved talent build: "..reason(op.name)
            lines[#lines+1]="Specialisation ID: "..op.expected.spec_id
            lines[#lines+1]="Copies the current committed talents into a NEW saved build."
            lines[#lines+1]="The native Talents UI will select the new build when ready."
            lines[#lines+1]="Expected new shared action bars: "..(op.expected_new_shared_action_bars and "Yes" or "No")
            lines[#lines+1]="This expectation is checked, not set. Actual action-bar contents are not verified and may change."
            lines[#lines+1]="Existing saved builds and current talent contents must remain unchanged."
            lines[#lines+1]="Open your Talents panel and close loadout dialogs before Apply. Keep Talents open until creation finishes."
            lines[#lines+1]="After completion, /aoexport within 60 seconds, then /reload. Do not repeat an uncertain creation."
        elseif op.kind=="rename_talent_config" or op.kind=="delete_talent_config" then
            local target=op.expected.entries[tostring(op.target_config_id)]
            lines[#lines+1]="Specialisation ID: "..op.expected.spec_id
            lines[#lines+1]="Saved loadout: "..reason(target.name).." ("..op.target_config_id..")"
            lines[#lines+1]="Selected saved loadout: "..(op.expected.selected_config_id or "None")
            if op.kind=="rename_talent_config" then
                lines[#lines+1]="New name: "..reason(op.name)
            else
                lines[#lines+1]="DELETE this saved talent loadout. Its saved contents will be removed."
                if op.target_config_id==op.expected.selected_config_id then
                    lines[#lines+1]="This is the selected saved build. Native UI must clear its saved selection; current talent contents must stay unchanged."
                end
                lines[#lines+1]="External references are not repaired. Recovery requires recreating the saved build."
            end
            lines[#lines+1]="Current talents and all other captured saved builds must remain unchanged."
            lines[#lines+1]="Actual action-bar contents are not verified."
            lines[#lines+1]="Open your Talents panel and close loadout edit/import dialogs before Apply."
            lines[#lines+1]="After completion, /aoexport within 60 seconds, then /reload. Do not repeat an uncertain operation."
        elseif op.kind=="load_talent_config" then
            lines[#lines+1]="Specialisation ID: "..op.spec_id
            lines[#lines+1]="Saved loadout: "..reason(op.desired_name).." ("..op.target_config_id..")"
            lines[#lines+1]="Expected selected loadout: "..op.expected_selected_config_id
            lines[#lines+1]="Shared action bars: "..(op.uses_shared_action_bars and "Yes" or "No — loadout-specific bars may change")
            lines[#lines+1]="Open your Talents panel before Apply. Current and saved build contents are checked again."
            lines[#lines+1]="Completion requires committed talents and matching saved selection."
            lines[#lines+1]="After the game finishes, run /aoexport within 60 seconds, then /reload. Do not repeat Apply while uncertain."
        elseif op.kind=="move_action_slot" then
            lines[#lines+1]="Source action slot: "..op.source_slot
            lines[#lines+1]="Target action slot: "..op.target_slot
            lines[#lines+1]=op.target.empty and "Move to captured empty slot." or "Swap the two captured actions."
            for _,entry in ipairs({{"Source",op.source},{"Target",op.target}}) do
                local action=entry[2]
                lines[#lines+1]=entry[1]..": "..(action.empty and "Empty" or (action.kind.." / "..action.id))
                if action.macro then
                    lines[#lines+1]="Macro name: "..action.macro.name
                    lines[#lines+1]="Macro icon: "..action.macro.icon
                    lines[#lines+1]="Macro body:\n"..action.macro.body
                end
            end
            lines[#lines+1]="Both slots and macro contents are checked again. No action body is executed."
            lines[#lines+1]="After completion, /aoexport within 60 seconds, then /reload. On failure inspect the cursor; do not retry."
        elseif op.kind=="move_bag_item" or op.kind=="split_bag_item" then
            lines[#lines+1]="Source bag/slot: "..op.bag.." / "..op.slot
            lines[#lines+1]="Target bag/slot: "..op.target_bag.." / "..op.target_slot
            lines[#lines+1]="Item ID: "..op.item_id.." / Count: "..op.count
            lines[#lines+1]="Source instance: "..op.source_guid
            lines[#lines+1]="Expected destination: empty (checked on Apply)"
            lines[#lines+1]=op.kind=="split_bag_item" and ("Split amount: "..op.amount.." / Remaining: "..(op.count-op.amount)) or "Moves the full stack."
            lines[#lines+1]="On failure, inspect the cursor before retrying."
        elseif op.kind=="equip_bag_item" then
            lines[#lines+1]="Source bag/slot: "..op.bag.." / "..op.slot
            lines[#lines+1]="Target equipment slot: "..op.target_slot
            lines[#lines+1]="Item ID: "..op.item_id
            lines[#lines+1]="Source instance: "..op.source_guid
            lines[#lines+1]="Expected target: "..(op.target_guid~="" and op.target_guid or "Empty (checked on Apply)")
            lines[#lines+1]="Displaced item returns to the source bag slot. On failure, inspect the cursor before any retry."
        elseif op.kind=="set_binding" then
            lines[#lines+1]="Key: "..op.key.." / Context: "..op.context
            lines[#lines+1]="Binding set: "..(op.binding_set==1 and "Account" or "Character")
            lines[#lines+1]="Expected action: "..(op.expected~="" and op.expected or "Unbound (checked on Apply)")
            lines[#lines+1]="Desired action: "..(op.desired~="" and op.desired or "Unbound")
        else
        for _,entry in ipairs({{"Expected",op.expected},{"Desired",op.desired}}) do
            if entry[2] then
                lines[#lines+1]=entry[1].." name: "..entry[2].name
                lines[#lines+1]="Icon: "..entry[2].icon
                lines[#lines+1]="Body:\n"..entry[2].body
            end
        end
        lines[#lines+1]="Action-bar references are not relocated."
        end
        details:SetText(table.concat(lines,"\n")); details:ClearFocus()
        status:SetText("Check the operation and scope above before applying.")
        apply:Enable()
    end)
    apply:SetScript("OnClick",function()
        apply:Disable()
        local ok,receipt,applied,detail=pcall(AO.ApplyReviewedMacro,input:GetText())
        if not ok then status:SetText("Refused: "..reason(receipt))
        elseif not applied then status:SetText("Outcome unknown: "..reason(detail).." Inspect current state before retrying.")
        elseif receipt.status=="outcome_unknown" then status:SetText("Operation unconfirmed. After it finishes, /aoexport within 60 seconds, then /reload. Do not retry.")
        else status:SetText("Result: "..receipt.status..". Export and reload to save a checkpoint.") end
    end)
    panel:SetScript("OnHide",function() AO.InvalidateMacroReview(); apply:Disable() end)
    input:SetFocus()
end
SLASH_AZEROTHOPERATORMACROS1="/aomacros"
SlashCmdList.AZEROTHOPERATORMACROS=open
SLASH_AZEROTHOPERATORMACROS2="/aoapply"
