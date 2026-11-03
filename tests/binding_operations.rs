use mlua::Lua;

#[test]
fn context_and_preimage_checked_before_binding_changes() -> mlua::Result<()> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='retail',interface=120105}; writes=0; saves=0; combat=false; binding='ACTIONBUTTON1'; set=2
        function InCombatLockdown() return combat end
        function GetBuildInfo() return '', '', '',120105 end
        function GetCurrentBindingSet() return set end
        function GetBindingAction(key,override,context) assert(key=='CTRL-1' and context==0 and override==false); return binding end
        function SetBinding(key,action,context) writes=writes+1; binding=action or ''; return true end
        function SaveBindings(which) assert(which==2); saves=saves+1 end
        C_KeyBindings={GetBindingContextForAction=function(a) return a=='HOUSING' and 1 or 0 end,
            GetBindingIndex=function(a) return a=='ACTIONBUTTON2' and 2 or nil end}
    "#).exec()?;
    let ao: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/BindingOperations.lua"))
        .call::<()>(("AzerothOperator", ao))?;
    lua.load(r#"
        local op={kind='set_binding',key='CTRL-1',expected='ACTIONBUTTON1',desired='ACTIONBUTTON2',binding_set=2,context=0}
        combat=true; assert(not pcall(AO.ApplyBindingOperation,op)); combat=false
        set=1; assert(not pcall(AO.ApplyBindingOperation,op)); set=2
        op.desired='HOUSING'; assert(not pcall(AO.ApplyBindingOperation,op))
        op.desired='UNKNOWN'; assert(not pcall(AO.ApplyBindingOperation,op))
        op.desired='ACTIONBUTTON2'; op.expected='stale'; assert(not pcall(AO.ApplyBindingOperation,op))
        assert(writes==0 and saves==0)
        op.expected='ACTIONBUTTON1'; assert(AO.ApplyBindingOperation(op).status=='applied')
        assert(writes==1 and saves==1 and binding=='ACTIONBUTTON2')
        op.expected=binding; op.desired=''; assert(AO.ApplyBindingOperation(op).status=='applied')
        assert(binding=='' and writes==2 and saves==2)
        op.expected=''; assert(AO.ApplyBindingOperation(op).status=='already_selected')
        assert(writes==2 and saves==2)
    "#).exec()
}
