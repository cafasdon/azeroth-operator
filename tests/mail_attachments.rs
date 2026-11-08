use azeroth_operator::import::decode;
use mlua::Lua;

#[test]
fn mail_capture_is_idle_partial_and_content_free() -> Result<(), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.load(r#"
        AO={family='forever',variant='live'};SlashCmdList={};mode='normal';reads=0
        function InCombatLockdown() return false end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function issecretvalue(v) return v=='PRIVATE-NAME' or (mode=='secret_id' and v==720) end
        MailFrame={IsVisible=function() return mode~='hidden' end,inboxBeingChecked=false}
        InboxFrame={IsVisible=function() return mode~='send_tab' end}
        C_Mail={IsCommandPending=function() return mode=='pending' end}
        function GetInboxNumItems()
            reads=reads+1
            if mode=='empty' then return 0,0 end
            if mode=='unloaded' then return 0,5 end
            if mode=='oversized' then return 101,101 end
            if mode=='bad_total' then return 2,1 end
            if mode=='changed' and reads>1 then return 1,5 end
            return 2,5
        end
        function HasInboxItem(message,slot) return (message==1 and slot<=2) or (message==2 and slot==16) end
        function GetInboxItem(message,slot)
            if message==2 then return nil,nil,nil,nil,nil,nil,false end
            if slot==2 then return 'PRIVATE-NAME',3,nil,4,nil,nil,true end
            local count=mode=='bad_count' and 0 or (mode=='changed_item' and reads>1 and 3 or 2)
            return 'PRIVATE-NAME',720,nil,count,nil,nil,false
        end
        function GetInboxHeaderInfo() error('header forbidden') end
        function GetInboxText() error('body forbidden') end
        function GetInboxInvoiceInfo() error('invoice forbidden') end
        function CheckInbox() error('refresh forbidden') end
        function TakeInboxItem() error('retrieval forbidden') end
        function TakeInboxMoney() error('retrieval forbidden') end
        function SendMail() error('send forbidden') end
        function DeleteInboxItem() error('delete forbidden') end
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua"))
        .call::<()>(("AzerothOperator", namespace.clone()))?;
    lua.load(include_str!("../addon/Core.lua"))
        .call::<()>(("AzerothOperator", namespace))?;
    let capture = || -> Result<azeroth_operator::Snapshot, Box<dyn std::error::Error>> {
        lua.load("reads=0;AO.Capture()").exec()?;
        let encoded: String = lua.globals().get("AzerothOperatorExport")?;
        Ok(decode(
            format!("AzerothOperatorExport = \"{encoded}\"").as_bytes(),
        )?)
    };
    let saved = capture()?;
    assert_eq!(
        saved.sections["mail_attachments"]["messages"]["1"]["1"],
        serde_json::json!({"kind":"item","item_id":720,"count":2})
    );
    assert_eq!(
        saved.sections["mail_attachments"]["messages"]["1"]["2"],
        serde_json::json!({"kind":"currency","currency_id":3,"count":4})
    );
    assert_eq!(
        saved.sections["mail_attachments"]["messages"]["2"]["16"],
        serde_json::json!({"kind":"unknown"})
    );
    assert!(!serde_json::to_string(&saved)?.contains("PRIVATE-NAME"));
    assert_eq!(
        azeroth_operator::analysis::summary(&saved)["mail_attachment_counts"],
        serde_json::json!({
        "loaded_messages":2,"total_messages":5,"observed_slots":3,"item_slots":1,"currency_slots":1,"unknown_slots":1})
    );
    for mode in [
        "hidden",
        "send_tab",
        "pending",
        "oversized",
        "bad_total",
        "changed",
        "changed_item",
        "bad_count",
        "secret_id",
    ] {
        lua.globals().set("mode", mode)?;
        assert!(
            capture()?.unavailable.contains_key("mail_attachments"),
            "{mode}"
        );
    }
    lua.load("mode='empty'").exec()?;
    assert_eq!(
        azeroth_operator::analysis::summary(&capture()?)["mail_attachment_counts"]["observed_slots"],
        0
    );
    lua.load("mode='unloaded'").exec()?;
    assert_eq!(capture()?.sections["mail_attachments"]["total_messages"], 5);
    lua.load("mode='normal';MailFrame.inboxBeingChecked=true")
        .exec()?;
    assert!(capture()?.unavailable.contains_key("mail_attachments"));
    Ok(())
}
