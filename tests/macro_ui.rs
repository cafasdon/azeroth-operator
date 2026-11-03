use mlua::Lua;

#[test]
fn player_review_apply_and_input_invalidation() -> mlua::Result<()> {
    let lua = Lua::new();
    lua.load(r#"
        AO={}; SlashCmdList={}; frames={}; applies=0; invalidations=0
        local function widget()
            local w={scripts={},enabled=true,text=''}
            setmetatable(w,{__index=function(_,k) return function() end end})
            function w:SetScript(k,f) self.scripts[k]=f end
            function w:SetText(t) self.text=t; if self.scripts.OnTextChanged then self.scripts.OnTextChanged(self) end end
            function w:GetText() return self.text end
            function w:Disable() self.enabled=false end
            function w:Enable() self.enabled=true end
            function w:CreateFontString() return widget() end
            return w
        end
        function CreateFrame(kind,name)
            local w=widget(); w.kind=kind; w.TitleText=widget(); frames[#frames+1]=w; return w
        end
        function AO.InvalidateMacroReview() invalidations=invalidations+1 end
        function AO.ReviewMacroImport(text)
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
        assert(not apply.enabled and applies==0)
        input:SetText('bad'); review.scripts.OnClick(); assert(not apply.enabled)
        input:SetText('valid'); review.scripts.OnClick(); assert(apply.enabled and applies==0)
        input:SetText('changed'); assert(not apply.enabled)
        input:SetText('valid'); review.scripts.OnClick(); apply.scripts.OnClick()
        assert(applies==1 and not apply.enabled)
        review.scripts.OnClick(); panel.scripts.OnHide(); assert(not apply.enabled)
        assert(invalidations>=5)
    "#,
    )
    .exec()
}
