use mlua::Lua;

#[test]
fn player_review_apply_and_input_invalidation() -> mlua::Result<()> {
    let lua = Lua::new();
    lua.load(r#"
        AO={}; SlashCmdList={}; frames={}; fonts={}; applies=0; invalidations=0
        local function widget()
            local w={scripts={},enabled=true,text=''}
            setmetatable(w,{__index=function(_,k) return function() end end})
            function w:SetScript(k,f) self.scripts[k]=f end
            function w:SetText(t) self.text=t; if self.scripts.OnTextChanged then self.scripts.OnTextChanged(self) end end
            function w:GetText() return self.text end
            function w:Disable() self.enabled=false end
            function w:Enable() self.enabled=true end
            function w:CreateFontString() local f=widget(); fonts[#fonts+1]=f; return f end
            function w:CreateTexture() return widget() end
            function w:SetHeight(height) self.height=height end
            function w:EnableMouse(enabled) self.mouse=enabled end
            function w:SetFocus() self.focused=true end
            return w
        end
        function CreateFrame(kind,name)
            local w=widget(); w.kind=kind; w.TitleText=widget(); frames[#frames+1]=w; return w
        end
        function AO.InvalidateMacroReview() invalidations=invalidations+1 end
        function AO.ReviewMacroImport(text)
            if text=='binding' then return {kind='set_binding',character='Test',realm='Realm',family='retail',variant='live',build='123',key='CTRL-1',context=0,binding_set=2,expected='',desired='ACTIONBUTTON1'} end
            if text=='bag' or text=='split' then return {kind=text=='split' and 'split_bag_item' or 'move_bag_item',character='Test',realm='Realm',family='retail',variant='live',build='123',bag=0,slot=1,target_bag=2,target_slot=3,item_id=123,count=20,amount=text=='split' and 5 or nil,source_guid='Item-Source'} end
            if text=='talent' then return {kind='load_talent_config',character='Test',realm='Realm',family='retail',variant='live',build='123',spec_id=263,target_config_id=9,expected_selected_config_id=8,desired_name='Saved Build',uses_shared_action_bars=false} end
            assert(text=='valid')
            return {kind='delete',character='Test',realm='Realm',family='retail',variant='live',build='123',source_scope='account',expected={name='Test',icon=1,body='/say test'}}
        end
        function AO.ApplyReviewedMacro(text) assert(text=='valid'); applies=applies+1; return {status='applied'},true end
    "#).exec()?;
    let ao: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/MacroUI.lua"))
        .call::<()>(("AzerothOperator", ao))?;
    lua.load(
        r#"
        assert(#frames==0)
        SlashCmdList.AZEROTHOPERATORMACROS()
        local panel,input,review,apply=frames[1],frames[3],frames[6],frames[7]
        assert(input.height==105 and input.mouse and input.focused)
        assert(not apply.enabled and applies==0)
        input:SetText('bad'); review.scripts.OnClick(); assert(not apply.enabled)
        input:SetText('valid'); review.scripts.OnClick(); assert(apply.enabled and applies==0)
        input:SetText('changed'); assert(not apply.enabled)
        input:SetText('valid'); review.scripts.OnClick(); apply.scripts.OnClick()
        assert(applies==1 and not apply.enabled)
        review.scripts.OnClick(); panel.scripts.OnHide(); assert(not apply.enabled)
        assert(invalidations>=5)
        input:SetText('binding'); review.scripts.OnClick()
        assert(apply.enabled and frames[5].text:find('Unbound (checked on Apply)',1,true))
        assert(frames[5].text:find('Binding set: Character',1,true))
        assert(frames[5].text:find('Desired action: ACTIONBUTTON1',1,true))
        input:SetText('bag'); review.scripts.OnClick()
        assert(apply.enabled and frames[5].text:find('Moves the full stack.',1,true))
        assert(frames[5].text:find('Target bag/slot: 2 / 3',1,true))
        input:SetText('split'); assert(not apply.enabled)
        review.scripts.OnClick()
        assert(apply.enabled and frames[5].text:find('Split amount: 5 / Remaining: 15',1,true))
        assert(frames[5].text:find('Source instance: Item-Source',1,true))
        assert(not frames[5].text:find('Moves the full stack.',1,true))
        input:SetText('talent'); review.scripts.OnClick()
        assert(apply.enabled and frames[5].text:find('Saved loadout: Saved Build (9)',1,true))
        assert(frames[5].text:find('loadout-specific bars may change',1,true))
        function AO.ApplyReviewedMacro() return {kind='load_talent_config',status='outcome_unknown'},true end
        apply.scripts.OnClick()
        assert(not apply.enabled and fonts[2].text:find('Talent load unconfirmed',1,true))

        function AO.ApplyReviewedMacro() return {status='outcome_unknown'},false,'Source |Hbad|h changed\ncheck' end
        apply.scripts.OnClick()
        assert(not apply.enabled and fonts[2].text:find('Outcome unknown:',1,true))
        assert(fonts[2].text:find('Source /Hbad/h changed check',1,true))
        review.scripts.OnClick()
        function AO.ApplyReviewedMacro() error('Different character') end
        apply.scripts.OnClick()
        assert(fonts[2].text:find('Refused:',1,true) and fonts[2].text:find('Different character',1,true))


    "#,
    )
    .exec()
}
