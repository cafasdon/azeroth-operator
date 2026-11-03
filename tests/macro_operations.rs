use mlua::Lua;

#[test]
fn real_lua_macro_mutations_and_precondition_refusals() -> mlua::Result<()> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='retail',interface=120105}; combat=false; writes=0
        Constants={MacroConsts={MAX_ACCOUNT_MACROS=120,MAX_CHARACTER_MACROS=30}}
        account={}; character={}; failDelete=false
        function InCombatLockdown() return combat end
        function GetBuildInfo() return '', '', '', 120105 end
        function GetNumMacros() return #account,#character end
        function GetMacroInfo(i)
            local m=i>120 and character[i-120] or account[i]
            if m then return m.name,m.icon,m.body end
        end
        function CreateMacro(n,i,b,c)
            writes=writes+1; local list=c and character or account
            list[#list+1]={name=n,icon=i,body=b}; return (c and 120 or 0)+#list
        end
        function EditMacro(slot,n,i,b)
            writes=writes+1; local list=slot>120 and character or account
            list[slot>120 and slot-120 or slot]={name=n,icon=i,body=b}
        end
        function DeleteMacro(slot)
            if failDelete then error('rejected') end
            writes=writes+1; table.remove(slot>120 and character or account,slot>120 and slot-120 or slot)
        end
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/MacroOperations.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    lua.load(r#"
        local a={name='AO Test',icon=134400,body='/say test'}
        local b={name='AO Edited',icon=134400,body='/say edited'}
        assert(AO.ApplyMacroOperation({kind='create',target_scope='account',desired=a}).status=='applied')
        local before=writes
        assert(not pcall(AO.ApplyMacroOperation,{kind='create',target_scope='account',desired=a}))
        combat=true
        assert(not pcall(AO.ApplyMacroOperation,{kind='delete',source_scope='account',expected=a}))
        combat=false
        assert(not pcall(AO.ApplyMacroOperation,{kind='delete',source_scope='account',expected=b}))
        assert(writes==before)
        assert(AO.ApplyMacroOperation({kind='edit',source_scope='account',expected=a,desired=b}).status=='applied')
        assert(AO.ApplyMacroOperation({kind='move',source_scope='account',target_scope='character',expected=b}).status=='applied')
        assert(#account==0 and #character==1)
        failDelete=true
        local partial=AO.ApplyMacroOperation({kind='move',source_scope='character',target_scope='account',expected=b})
        assert(partial.status=='copied_source_removal_unconfirmed')
        assert(#account==1 and #character==1)
        failDelete=false
        assert(AO.ApplyMacroOperation({kind='delete',source_scope='account',expected=b}).status=='applied')
        assert(#account==0 and #character==1)
        AO.family='classic'; before=writes
        assert(not pcall(AO.ApplyMacroOperation,{kind='delete',source_scope='character',expected=b}))
        assert(writes==before)
    "#).exec()
}
