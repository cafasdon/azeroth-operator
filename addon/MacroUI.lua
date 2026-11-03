local _, AO = ...
local panel
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
    panel.TitleText:SetText("Azeroth Operator — macro operation")
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
    input:SetMaxLetters(8192)
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
        for _,entry in ipairs({{"Expected",op.expected},{"Desired",op.desired}}) do
            if entry[2] then
                lines[#lines+1]=entry[1].." name: "..entry[2].name
                lines[#lines+1]="Icon: "..entry[2].icon
                lines[#lines+1]="Body:\n"..entry[2].body
            end
        end
        lines[#lines+1]="Action-bar references are not relocated."
        details:SetText(table.concat(lines,"\n")); details:ClearFocus()
        status:SetText("Check the operation above. Apply changes this character's macros.")
        apply:Enable()
    end)
    apply:SetScript("OnClick",function()
        apply:Disable()
        local ok,receipt,applied=pcall(AO.ApplyReviewedMacro,input:GetText())
        if not ok then status:SetText("Refused. Check character, client, combat and previous attempts.")
        elseif not applied then status:SetText("Outcome unknown. Inspect macros before preparing another operation.")
        else status:SetText("Result: "..receipt.status..". Export and reload to save a checkpoint.") end
    end)
    panel:SetScript("OnHide",function() AO.InvalidateMacroReview(); apply:Disable() end)
    input:SetFocus()
end
SLASH_AZEROTHOPERATORMACROS1="/aomacros"
SlashCmdList.AZEROTHOPERATORMACROS=open
