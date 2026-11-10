use azeroth_operator::{Snapshot, import::decode};
use mlua::Lua;
use serde_json::json;

fn capture(mode: &str) -> Result<(Snapshot, Lua), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.globals().set("mode", mode)?;
    lua.load(r#"
        AO={family='forever',variant='live'};SlashCmdList={};reads=0;counts=0;actions=0;SECRET={}
        function issecretvalue(v) return rawequal(v,SECRET) end
        function InCombatLockdown() return mode=='combat' end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName(unit) assert(unit=='player');return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function print() end
        function forbidden() actions=actions+1;error('Forbidden mail action') end
        MailFrame={inboxBeingChecked=false,IsVisible=function() return mode~='hidden' end}
        InboxFrame={IsVisible=function() return mode~='hidden_inbox' end}
        if mode=='inherited_methods' then
            local mailVisible,inboxVisible=MailFrame.IsVisible,InboxFrame.IsVisible
            MailFrame.IsVisible=nil;InboxFrame.IsVisible=nil
            setmetatable(MailFrame,{__index={IsVisible=mailVisible}})
            setmetatable(InboxFrame,{__index={IsVisible=inboxVisible}})
        end
        C_Mail={IsCommandPending=function() return mode=='pending' end}
        if mode=='pending_hides_frame' then
            local hidden=false
            MailFrame.IsVisible=function() return not hidden end
            C_Mail.IsCommandPending=function()
                if counts>=2 then hidden=true end
                return false
            end
        end
        if mode=='visibility_starts_command' then
            local busy=false
            MailFrame.IsVisible=function()
                if counts>=2 then busy=true end
                return true
            end
            C_Mail.IsCommandPending=function() return busy end
        end
        function GetInboxNumItems()
            counts=counts+1
            if mode=='empty' then return 0,0 end
            if mode=='changed_count' and counts>1 then return 2,5 end
            if mode=='over_limit' then return 101,101 end
            return 3,5
        end
        local perSlot={}
        function GetInboxHeaderInfo(slot)
            assert(slot>=1 and slot<=3);reads=reads+1;perSlot[slot]=(perSlot[slot] or 0)+1
            if mode=='header_error' then error('PRIVATE_PROVIDER_MESSAGE') end
            local money,cod=slot==1 and 100 or 0,slot==2 and 20 or 0
            if mode=='changed_amount' and slot==1 and perSlot[slot]>1 then money=200 end
            if mode=='partial' and slot==3 then money=nil end
            if mode=='restricted' and slot==3 then money=SECRET end
            if mode=='fractional' and slot==3 then money=0.5 end
            if mode=='negative' and slot==3 then cod=-1 end
            if mode=='string' and slot==3 then money='PRIVATE' end
            if mode=='too_large' and slot==3 then money=10000000000000 end
            if mode=='maximum' then money=9999999999999;cod=9999999999999 end
            if mode=='replace_api' then GetInboxHeaderInfo=function() return nil end end
            if mode=='replace_frame' then MailFrame={inboxBeingChecked=false,IsVisible=function() return true end} end
            if mode=='replace_family' then AO.family='retail' end
            if mode=='replace_guard' then issecretvalue=function(v) return rawequal(v,SECRET) end end
            if mode=='busy_during_read' then MailFrame.inboxBeingChecked=true end
            return 'PRIVATE_ICON','PRIVATE_STATIONERY',SECRET,'PRIVATE_SUBJECT',money,cod,
                'PRIVATE_DAYS','PRIVATE_ITEMCOUNT','PRIVATE_READ',SECRET
        end
        if mode=='missing' then GetInboxHeaderInfo=nil end
        if mode=='checking' then MailFrame.inboxBeingChecked=true end
        if mode=='unknown_checking' then MailFrame.inboxBeingChecked=nil end
        CheckInbox=forbidden;AutoLootMailItem=forbidden;TakeInboxMoney=forbidden;TakeInboxItem=forbidden
        DeleteInboxItem=forbidden;ReturnInboxItem=forbidden;SendMail=forbidden
        GetInboxText=forbidden;GetInboxInvoiceInfo=forbidden;GetMoney=forbidden
    "#).exec()?;
    let ns: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", ns.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", ns))?;
    lua.load("AO.Capture()").exec()?;
    let exported: String = lua.globals().get("AzerothOperatorExport")?;
    let saved = decode(format!("AzerothOperatorExport = \"{exported}\"").as_bytes())?;
    assert!(!serde_json::to_string(&saved)?.contains("PRIVATE"));
    assert_eq!(lua.globals().get::<u32>("actions")?, 0);
    Ok((saved, lua))
}

#[test]
fn loaded_numeric_funds_survive_without_correspondence_or_wallet_credit()
-> Result<(), Box<dyn std::error::Error>> {
    let (saved, lua) = capture("normal")?;
    assert_eq!(
        saved.sections["mail_funds"],
        json!({"coverage":"loaded_inbox_funds","loaded_messages":3,
        "total_messages":5,"messages":{"1":{"status":"reported","money_copper":100,"cod_copper":0},
            "2":{"status":"reported","money_copper":0,"cod_copper":20},
            "3":{"status":"reported","money_copper":0,"cod_copper":0}}})
    );
    assert_eq!(lua.globals().get::<u32>("reads")?, 6);
    assert!(!saved.sections.contains_key("wallet"));
    let (inherited, _) = capture("inherited_methods")?;
    assert_eq!(
        inherited.sections["mail_funds"],
        saved.sections["mail_funds"]
    );
    let (maximum, _) = capture("maximum")?;
    assert_eq!(
        maximum.sections["mail_funds"]["messages"]["1"]["money_copper"],
        9999999999999u64
    );
    let (empty, lua) = capture("empty")?;
    assert_eq!(empty.sections["mail_funds"]["messages"], json!({}));
    assert_eq!(lua.globals().get::<u32>("reads")?, 0);
    Ok(())
}

#[test]
fn partial_or_unsafe_reads_stay_unknown_or_unavailable() -> Result<(), Box<dyn std::error::Error>> {
    for mode in [
        "partial",
        "restricted",
        "fractional",
        "negative",
        "string",
        "too_large",
    ] {
        let (saved, _) = capture(mode)?;
        assert_eq!(
            saved.sections["mail_funds"]["messages"]["3"],
            json!({"status":"unavailable"}),
            "{mode}"
        );
        assert_eq!(
            saved.sections["mail_funds"]["messages"]["1"]["money_copper"],
            100
        );
    }
    let (changed, _) = capture("changed_amount")?;
    assert_eq!(
        changed.sections["mail_funds"]["messages"]["1"],
        json!({"status":"unavailable"})
    );
    let (errors, _) = capture("header_error")?;
    assert!(
        errors.sections["mail_funds"]["messages"]
            .as_object()
            .unwrap()
            .values()
            .all(|row| row == &json!({"status":"unavailable"}))
    );
    for mode in [
        "hidden",
        "hidden_inbox",
        "pending",
        "checking",
        "unknown_checking",
        "changed_count",
        "over_limit",
        "missing",
        "replace_api",
        "replace_frame",
        "replace_family",
        "replace_guard",
        "busy_during_read",
        "pending_hides_frame",
        "visibility_starts_command",
    ] {
        let (saved, _) = capture(mode)?;
        assert!(!saved.sections.contains_key("mail_funds"), "{mode}");
        assert_eq!(saved.unavailable["mail_funds"], "api_unavailable_or_failed");
    }
    assert!(capture("combat").is_err());
    Ok(())
}
