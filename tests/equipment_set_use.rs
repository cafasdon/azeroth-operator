use mlua::Lua;

#[test]
fn native_set_use_checks_exact_equipment_and_never_retries() -> mlua::Result<()> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='retail',variant='live',interface=120105,collectors={}}
        function GetBuildInfo() return '', '70688', '', 120105 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Fixture' end
        function InCombatLockdown() return combat end
        function UnitCastingInfo() return casting end
        function UnitChannelInfo() return channel end
        function GetCursorInfo() return cursor end
        function IsInventoryItemLocked(i) return locked==i end
        function copy(t) local out={} for k,v in pairs(t) do out[k]=type(v)=='table' and copy(v) or v end return out end
        ItemLocation={CreateFromEquipmentSlot=function(_,i) return i end}
        C_Item={DoesItemExist=function(i) return worn[tostring(i)]~='' end,
            GetItemGUID=function(i) local g=worn[tostring(i)]; if g~='' then return g end end}
        function finish() worn['16']='Item-target' end
        local native=function(id)
            assert(id==0); writes=writes+1
            if mode=='throw' then error('native failure') end
            if mode=='partial' then worn['16']='Item-wrong'; return true end
            if mode=='ignored' then finish(); worn['17']='Item-unexpected'; return true end
            if mode=='cursor' then finish(); cursor='item'; return true end
            if mode=='delete' then finish(); sets={}; return true end
            if mode=='delayed' or mode=='reject' then return true end
            finish(); return false -- a return value alone does not establish the outcome
        end
        function reset()
            combat=false; casting=nil; channel=nil; cursor=nil; locked=nil; writes=0; mode=nil; setLocked=false
            worn={}; local slots={}
            for i=1,19 do worn[tostring(i)]=''; slots[tostring(i)]={ignored=true} end
            worn['16']='Item-old'
            slots['16']={ignored=false,raw_item_id=123,raw_location=100,
                observed_item={kind='bag',bag=0,slot=1,item_id=123,guid='Item-target',locked=false}}
            sets={['0']={name='Fixture set',icon=123,num_lost=0,assigned_spec=2,slots=slots}}
            op={kind='use_equipment_set',family='retail',variant='live',build='70688',interface=120105,
                character='Fixture',realm='Fixture',set_id=0,expected=copy(sets['0']),equipment_before=copy(worn)}
            C_EquipmentSet={UseEquipmentSet=native,EquipmentSetContainsLockedItems=function() return setLocked end}
        end
        function AO.collectors.equipment_sets()
            return {coverage='native_catalogue_raw_slots',can_use=true,entries=copy(sets)}
        end
        reset()
    "#).exec()?;
    let ao: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/EquipmentSetUse.lua"))
        .call::<()>(("AzerothOperator", ao))?;
    lua.load(
        r#"
        assert(AO.ApplyEquipmentSetUse(op).status=='applied' and writes==1)
        assert(worn['16']=='Item-target' and worn['17']=='')
        assert(not pcall(AO.ApplyEquipmentSetUse,op) and writes==1)
        reset(); worn['16']='Item-target'; op.equipment_before=copy(worn)
        assert(AO.ApplyEquipmentSetUse(op).status=='already_selected' and writes==0)
        for _,modeValue in ipairs({'partial','ignored','cursor','delete','delayed','reject'}) do
            reset(); mode=modeValue
            assert(AO.ApplyEquipmentSetUse(op).status=='outcome_unknown' and writes==1,modeValue)
        end
        reset(); mode='delayed'; assert(AO.ApplyEquipmentSetUse(op).status=='outcome_unknown')
        finish(); assert(AO.VerifyEquipmentSetUse(op) and writes==1)
        sets['0'].assigned_spec=1; assert(not AO.VerifyEquipmentSetUse(op))
        reset(); mode='throw'; assert(not pcall(AO.ApplyEquipmentSetUse,op) and writes==1)
        local failures={
            function() combat=true end,
            function() casting='cast' end,
            function() channel='channel' end,
            function() cursor='item' end,
            function() locked=17 end,
            function() setLocked=true end,
            function() C_EquipmentSet.UseEquipmentSet=nil end,
            function() C_EquipmentSet.EquipmentSetContainsLockedItems=nil end,
            function() op.build='wrong' end,
            function() op.family='classic' end,
            function() op.equipment_before['19']=nil end,
            function() worn['17']='Item-player-edit' end,
            function() sets['0'].slots['16'].observed_item.guid='Item-other-copy' end,
            function() op.expected.num_lost=1 end,
            function() op.expected.slots['16'].ignored=nil end,
            function() op.expected.slots['16'].observed_item=nil end,
            function() op.expected.slots['16'].observed_item.locked=true end,
            function() op.expected.slots['16'].raw_item_id=0 end,
            function() op.expected.slots['16'].raw_item_id=-1 end,
            function() op.expected.slots['17']=copy(op.expected.slots['16']) end,
        }
        for i,f in ipairs(failures) do
            reset(); f(); assert(not pcall(AO.ApplyEquipmentSetUse,op) and writes==0,'guard '..i)
        end
        reset(); issecretvalue=function(v) return v=='Item-old' end
        assert(not pcall(AO.ApplyEquipmentSetUse,op) and writes==0); issecretvalue=nil
        reset(); local get=C_Item.GetItemGUID; local n=0
        C_Item.GetItemGUID=function(i) n=n+1; if n==2 then return 'Item-race' end return get(i) end
        assert(not pcall(AO.ApplyEquipmentSetUse,op) and writes==0); C_Item.GetItemGUID=get
    "#,
    )
    .exec()
}
