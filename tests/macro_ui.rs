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
            function w:SetMaxLetters(n) self.maxLetters=n end
            function w:EnableMouse(enabled) self.mouse=enabled end
            function w:SetFocus() self.focused=true end
            return w
        end
        function CreateFrame(kind,name)
            local w=widget(); w.kind=kind; w.TitleText=widget(); frames[#frames+1]=w; return w
        end
        function AO.InvalidateMacroReview() invalidations=invalidations+1 end
        function AO.ReviewMacroImport(text)
            if text=='talent_import' then return {kind='import_talent_config',character='Test',realm='Realm',family='retail',variant='live',build='123',name='New|Import',import_string='ABCDE',allow_empty_hash=true,expected_new_shared_action_bars=false,expected={spec_id=263}} end
            if text=='talent_create' then return {kind='create_talent_config',character='Test',realm='Realm',family='retail',variant='live',build='123',name='New|Build',expected_new_shared_action_bars=false,expected={spec_id=263}} end
            if text=='selected_delete' then return {kind='delete_talent_config',character='Test',realm='Realm',family='retail',variant='live',build='123',target_config_id=9,expected={spec_id=263,selected_config_id=9,entries={['9']={name='Selected'}}}} end
            if text=='talent_rename' or text=='talent_delete' then
                return {kind=text=='talent_rename' and 'rename_talent_config' or 'delete_talent_config',character='Test',realm='Realm',family='retail',variant='live',build='123',target_config_id=9,name='New build',expected={spec_id=263,selected_config_id=8,entries={['9']={name='Old|Build'}}}}
            end
            if text=='set_create' or text=='set_save' then
                local slots,before,ignored={},{},{}
                for i=1,19 do local k=tostring(i); slots[k]={ignored=true}; before[k]={guid='',item_id=0}; ignored[k]=true end
                before['16']={guid='Item|exact',item_id=123}; ignored['16']=false
                return {kind=text=='set_create' and 'create_equipment_set' or 'save_equipment_set',
                    character='Test',realm='Realm',family='retail',variant='live',build='123',
                    set_id=text=='set_save' and 0 or nil,name='Test|Set',icon=123,equipment_before=before,ignored_slots=ignored,
                    expected_catalogue={['0']={name='Test|Set',assigned_spec=2,slots=slots}}}
            end
            if text=='spec_assign' or text=='spec_unassign' then
                return {kind=text=='spec_assign' and 'assign_equipment_set_spec' or 'unassign_equipment_set_spec',
                    character='Test',realm='Realm',family='retail',variant='live',build='123',set_id=1,
                    spec_index=text=='spec_assign' and 2 or nil,spec_id=text=='spec_assign' and 262 or nil,
                    displaced_set_id=text=='spec_assign' and 0 or nil,
                    expected_catalogue={['1']={name='Target',assigned_spec=3},['0']={name='Other|Set',assigned_spec=2}}}
            end
            if text=='set_use' then
                local slots,before={},{}
                for i=1,19 do slots[tostring(i)]={ignored=true}; before[tostring(i)]='' end
                slots['16']={ignored=false,observed_item={guid='Item|target'}}; before['16']='Item-old'
                return {kind='use_equipment_set',character='Test',realm='Realm',family='retail',variant='live',build='123',
                    set_id=0,equipment_before=before,expected={name='Equip set',icon=123,num_items=1,num_lost=0,num_ignored=18,slots=slots}}
            end
            if text=='set_modify' or text=='set_delete' then return {kind=text=='set_modify' and 'modify_equipment_set' or 'delete_equipment_set',character='Test',realm='Realm',family='retail',variant='live',build='123',set_id=0,name='New Set',icon=123,expected={name='Old|Set',icon=134400,assigned_spec=2,num_items=18,num_lost=1,num_ignored=1}} end
            if text=='binding' then return {kind='set_binding',character='Test',realm='Realm',family='retail',variant='live',build='123',key='CTRL-1',context=0,binding_set=2,expected='',desired='ACTIONBUTTON1'} end
            if text=='bag' or text=='split' then return {kind=text=='split' and 'split_bag_item' or 'move_bag_item',character='Test',realm='Realm',family='retail',variant='live',build='123',bag=0,slot=1,target_bag=2,target_slot=3,item_id=123,count=20,amount=text=='split' and 5 or nil,source_guid='Item-Source'} end
            if text=='talent' then return {kind='load_talent_config',character='Test',realm='Realm',family='retail',variant='live',build='123',spec_id=263,target_config_id=9,expected_selected_config_id=8,desired_name='Saved Build',uses_shared_action_bars=false} end
            if text=='action' then return {kind='move_action_slot',character='Test',realm='Realm',family='retail',variant='live',build='123',source_slot=1,target_slot=2,source={empty=false,kind='macro',id=121,macro={name='My Macro',icon=134400,body='/cast Test'}},target={empty=true}} end
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
        assert(input.maxLetters==524288)
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
        input:SetText('action'); review.scripts.OnClick()
        assert(apply.enabled and frames[5].text:find('Move to captured empty slot.',1,true))
        assert(frames[5].text:find('Source action slot: 1',1,true) and frames[5].text:find('Macro name: My Macro',1,true))
        assert(frames[5].text:find('/cast Test',1,true))
        input:SetText('set_modify'); review.scripts.OnClick()
        assert(apply.enabled and frames[5].text:find('Equipment set: Old/Set (ID 0)',1,true))
        assert(frames[5].text:find('New name: New Set',1,true) and frames[5].text:find('New icon: 123',1,true))
        assert(frames[5].text:find('Assigned specialisation index: 2',1,true))
        input:SetText('set_delete'); review.scripts.OnClick()
        assert(apply.enabled and frames[5].text:find('DELETE this saved equipment set',1,true))
        assert(frames[5].text:find('This does not delete the items.',1,true))
        assert(frames[5].text:find('External references are not repaired.',1,true))
        input:SetText('set_use'); review.scripts.OnClick()
        assert(apply.enabled and frames[5].text:find('Equip this saved set.',1,true))
        assert(frames[5].text:find('Slot 16 / Equip: Item-old -> Item/target',1,true))
        assert(frames[5].text:find('Slot 19 / Preserve: Empty (checked on Apply) -> Empty',1,true))
        assert(frames[5].text:find('A partial swap remains unconfirmed.',1,true))
        assert(not frames[5].text:find('DELETE this saved',1,true))
        input:SetText('set_create'); review.scripts.OnClick()
        assert(apply.enabled and frames[5].text:find('Equipment set: Test/Set (new)',1,true))
        assert(frames[5].text:find('Create a new set',1,true) and frames[5].text:find('Save item 123 / Equipped: Item/exact',1,true))
        assert(frames[5].text:find('Ignore in saved set / Equipped: Empty (checked on Apply)',1,true))
        input:SetText('set_save'); review.scripts.OnClick()
        assert(apply.enabled and frames[5].text:find('Equipment set: Test/Set (ID 0)',1,true))
        assert(frames[5].text:find('OVERWRITE',1,true) and frames[5].text:find('Preserved specialisation index: 2',1,true))
        assert(frames[5].text:find('Close the native equipment-set editor',1,true))
        input:SetText('spec_assign'); review.scripts.OnClick()
        assert(apply.enabled and frames[5].text:find('Assign to specialisation index 2 / ID 262',1,true))
        assert(frames[5].text:find('Remove this association from: Other/Set (ID 0)',1,true))
        assert(frames[5].text:find('Future specialisation changes may select different equipment.',1,true))
        input:SetText('spec_unassign'); review.scripts.OnClick()
        assert(apply.enabled and frames[5].text:find("Remove this set's specialisation association.",1,true))
        assert(not frames[5].text:find('Remove this association from:',1,true))
        input:SetText('talent_import'); review.scripts.OnClick()
        assert(apply.enabled and frames[5].text:find('CHANGING YOUR ACTIVE TALENTS',1,true))
        assert(frames[5].text:find('Import text: ABCDE',1,true) and frames[5].text:find('Omitted tree hash allowed: Yes',1,true))
        assert(frames[5].text:find('New/Import',1,true) and frames[5].text:find('Do not repeat an uncertain import',1,true))
        input:SetText('talent_create'); review.scripts.OnClick()
        assert(apply.enabled and frames[5].text:find('Create saved talent build: New/Build',1,true))
        assert(frames[5].text:find('will select the new build',1,true) and frames[5].text:find('checked, not set',1,true))
        assert(frames[5].text:find('may change',1,true) and frames[5].text:find('Do not repeat an uncertain creation',1,true))
        input:SetText('talent_rename'); review.scripts.OnClick()
        assert(apply.enabled and frames[5].text:find('New name: New build',1,true))
        assert(frames[5].text:find('Old/Build',1,true) and frames[5].text:find('Actual action-bar contents are not verified.',1,true))
        input:SetText('selected_delete'); review.scripts.OnClick()
        assert(apply.enabled and frames[5].text:find('Native UI must clear its saved selection',1,true))
        input:SetText('talent_delete'); review.scripts.OnClick()
        assert(apply.enabled and frames[5].text:find('DELETE this saved talent loadout',1,true))
        assert(frames[5].text:find('Recovery requires recreating',1,true))
        input:SetText('talent'); review.scripts.OnClick()
        assert(apply.enabled and frames[5].text:find('Saved loadout: Saved Build (9)',1,true))
        assert(frames[5].text:find('loadout-specific bars may change',1,true))
        function AO.ApplyReviewedMacro() return {kind='load_talent_config',status='outcome_unknown'},true end
        apply.scripts.OnClick()
        assert(not apply.enabled and fonts[2].text:find('Operation unconfirmed',1,true))

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
