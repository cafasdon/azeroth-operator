use mlua::Lua;

#[test]
fn cooldown_tuple_capture_is_optional_public_and_stable() -> Result<(), Box<dyn std::error::Error>>
{
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever'};mode='normal';cooldownReads=0
        function issecretvalue(v) return v=='PRIVATE' end
        Enum={TradeskillRecipeType={Item=1,Enchant=2}}
        recipeInfo={recipeID=2675,name='Fixture',learned=true}
        schematic={recipeID=2675,isRecraft=false,recipeType=1,outputItemID=2870,
            quantityMin=1,quantityMax=1,reagentSlotSchematics={
                {required=true,reagents={{itemID=2840}},variableQuantities={},quantityRequired=5}}}
        local form={IsVisible=function() return true end,GetRecipeInfo=function() return recipeInfo end,
            recipeSchematic=schematic}
        ProfessionsFrame={CraftingPage={IsVisible=function() return mode~='hidden' end,SchematicForm=form}}
        Professions={InLocalCraftingMode=function() return mode~='nonlocal' end,
            IsViewingExternalCraftingList=function() return mode=='external' end,
            GetProfessionInfo=function()
                return {professionID=mode=='changed_context' and cooldownReads>0 and 165 or 164}
            end}
        C_TradeSkillUI={IsNPCCrafting=function() return mode=='npc' end,
            IsDataSourceChanging=function() return mode=='changing' or (mode=='became_changing' and cooldownReads>0) end,
            GetProfessionSpells=function()
                if mode=='changed_ids' and cooldownReads>0 then return {2675,3508} end
                return {2675}
            end,
            GetRecipeInfo=function(id) assert(id==2675);return recipeInfo end,
            GetRecipeSchematic=function(id,recraft) assert(id==2675 and recraft==false);return schematic end,
            GetCraftableCount=function(id) assert(id==2675);return 7 end,
            GetProfessionInfoByRecipeID=function() return nil end,
            GetRecipeRequirements=function() return nil end,
            CraftRecipe=function() error('no crafting') end,
            SetShowLearned=function() error('no filters') end,
            SetShowUnlearned=function() error('no filters') end,
            RefreshRecipes=function() error('no refresh') end}
        function reset(getter)
            cooldownReads=0
            C_TradeSkillUI.GetRecipeCooldown=getter and function(id)
                assert(id==2675, 'cooldown must use the same recipe ID')
                cooldownReads=cooldownReads+1
                return getter(cooldownReads)
            end or nil
        end
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    lua.load(r#"
        local function equal(a,b)
            if type(a)~=type(b) then return false end
            if type(a)~='table' then return a==b end
            for key,value in pairs(a) do if not equal(value,b[key]) then return false end end
            for key in pairs(b) do if a[key]==nil then return false end end
            return true
        end
        local collectors={
            bulk=function() return AO.collectors.profession_recipe_materials().entries['2675'] end,
            selected=AO.collectors.selected_recipe}
        local valid={
            {'tuple order',function() return 12.1,false,1,3 end,
                {remaining_seconds=13,daily=false,charges=1,max_charges=3}},
            {'zero',function() return 0,false,0,3 end,
                {remaining_seconds=0,daily=false,charges=0,max_charges=3}},
            {'daily',function() return 0,true end,{remaining_seconds=0,daily=true}},
            {'seconds only',function() return 0.01 end,{remaining_seconds=1}},
            {'charges only',function() return nil,nil,2,3 end,{charges=2,max_charges=3}},
            {'charges with daily flag',function() return nil,false,2,3 end,{charges=2,max_charges=3}},
            {'maximum',function() return 4294967295,nil,4294967295,4294967295 end,
                {remaining_seconds=4294967295,charges=4294967295,max_charges=4294967295}},
            {'same normalised seconds',function(n) return 12.9-n/10 end,{remaining_seconds=13}}}
        local optional={
            {'missing',nil},
            {'nil',function() return nil end},
            {'daily alone',function() return nil,false end},
            {'throw',function() error('PRIVATE') end},
            {'throw on second read',function(n) if n==2 then error('PRIVATE') end;return 0 end},
            {'secret seconds',function() return 'PRIVATE',false,1,3 end},
            {'secret daily',function() return 0,'PRIVATE',1,3 end},
            {'secret charges',function() return 0,false,'PRIVATE',3 end},
            {'secret maximum',function() return 0,false,1,'PRIVATE' end},
            {'negative seconds',function() return -1 end},
            {'large seconds',function() return 4294967296 end},
            {'infinite seconds',function() return math.huge end},
            {'nan seconds',function() return 0/0 end},
            {'boolean seconds',function() return false end},
            {'table seconds',function() return {} end},
            {'string seconds',function() return '12' end},
            {'invalid daily',function() return 0,1,1,3 end},
            {'negative charges',function() return 0,false,-1,3 end},
            {'fractional charges',function() return 0,false,1.5,3 end},
            {'fractional maximum',function() return 0,false,1,3.5 end},
            {'large maximum',function() return 0,false,1,4294967296 end},
            {'zero maximum',function() return 0,false,0,0 end},
            {'over maximum',function() return 0,false,4,3 end},
            {'missing charges',function() return 0,false,nil,3 end},
            {'missing maximum',function() return 0,false,1 end},
            {'boolean charges',function() return 0,false,false,3 end},
            {'nan charges',function() return 0,false,0/0,3 end},
            {'infinite maximum',function() return 0,false,1,math.huge end},
            {'moving timer',function(n) return 13-n end},
            {'moving charges',function(n) return 0,false,n,3 end},
            {'moving daily',function(n) return 0,n==1 end},
            {'moving maximum',function(n) return 0,false,0,n end},
            {'disappearing seconds',function(n) if n==1 then return 0 end end}}
        -- A bad optional component does not suppress other public, valid components.
        local partial={
            ['secret seconds']={charges=1,max_charges=3},
            ['secret daily']={remaining_seconds=0,charges=1,max_charges=3},
            ['invalid daily']={remaining_seconds=0,charges=1,max_charges=3}}
        for _,label in ipairs({'secret charges','secret maximum','negative charges','fractional charges',
            'fractional maximum','large maximum','zero maximum','over maximum','missing charges',
            'missing maximum','boolean charges','nan charges','infinite maximum'}) do
            partial[label]={remaining_seconds=0,daily=false}
        end
        for name,collector in pairs(collectors) do
            for _,case in ipairs(valid) do
                reset(case[2]);local row=collector()
                assert(equal(row.cooldown,case[3]),name..': '..case[1])
                assert(row.reagents['2840']==5)
                assert(cooldownReads==2)
                if name=='bulk' then assert(row.reported_craftable_count==7) end
            end
            for _,case in ipairs(optional) do
                reset(case[2]);local row=collector()
                assert(equal(row.cooldown,partial[case[1]]),name..': '..case[1])
                assert(row.reagents['2840']==5)
                assert(cooldownReads==(case[2] and 2 or 0))
                if name=='bulk' then assert(row.status=='fixed_items' and row.reported_craftable_count==7) end
            end
            C_TradeSkillUI.GetRecipeCooldown='PRIVATE'
            assert(collector().cooldown==nil)
            C_TradeSkillUI.GetRecipeCooldown=false
            assert(collector().cooldown==nil)
            reset(nil)
            setmetatable(C_TradeSkillUI,{__index=function(_,key)
                if key=='GetRecipeCooldown' then error('PRIVATE optional getter unavailable') end
            end})
            local ok,row=pcall(collector)
            setmetatable(C_TradeSkillUI,nil)
            assert(ok,name..': optional getter lookup must preserve recipe')
            assert(row.cooldown==nil and row.reagents['2840']==5)
        end
        reset(function() return 0 end);recipeInfo.learned=false
        assert(AO.collectors.selected_recipe().cooldown==nil and cooldownReads==0)
        assert(AO.collectors.profession_recipe_materials().entries['2675'].status=='unavailable' and cooldownReads==0)
        recipeInfo.learned=true
        schematic.reagentSlotSchematics[1].reagents[2]={itemID=2841}
        local unsupported=AO.collectors.profession_recipe_materials().entries['2675']
        assert(unsupported.status=='unsupported_required_slots' and unsupported.cooldown==nil and cooldownReads==0)
        schematic.reagentSlotSchematics[1].reagents[2]=nil
        for _,value in ipairs({'hidden','nonlocal','external','npc','changing','changed_context','changed_ids','became_changing'}) do
            mode=value;reset(function() return 0 end)
            assert(not pcall(AO.collectors.profession_recipe_materials),value..' must discard changed context')
        end
        mode='normal';ProfessionsFrame.CraftingPage.SchematicForm.isInspection=true
        assert(not pcall(AO.collectors.profession_recipe_materials))
    "#).exec()?;
    Ok(())
}
