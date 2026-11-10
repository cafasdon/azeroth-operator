use azeroth_operator::{Snapshot, import::decode};
use mlua::Lua;
use serde_json::json;

fn capture(mode: &str) -> Result<(Snapshot, Lua), Box<dyn std::error::Error>> {
    let lua = Lua::new();
    lua.globals().set("mode", mode)?;
    lua.load(r#"
        AO={family='forever',variant='live'}; SlashCmdList={}; reads=0;counts=0;actions=0
        SECRET={}
        function issecretvalue(v) return rawequal(v,SECRET) end
        function InCombatLockdown() return false end
        function GetBuildInfo() return '1.60.1','70862','',16001 end
        function UnitName() return 'Fixture' end
        function GetRealmName() return 'Test' end
        function GetServerTime() return 1735689600 end
        function date() return '2025-01-01T00:00:00Z' end
        function print() end
        function forbidden() actions=actions+1; error('forbidden auction action') end
        AuctionHouseFrame={IsShown=function() return mode~='hidden' end}
        C_AuctionHouse={}
        function C_AuctionHouse.GetNumOwnedAuctions()
            counts=counts+1
            if mode=='empty' then return 0 end
            if mode=='over_budget' then return 257 end
            if mode=='changing_count' and counts>1 then return 1 end
            return mode=='maximum' and 256 or 2
        end
        function C_AuctionHouse.GetOwnedAuctionInfo(slot)
            reads=reads+1
            local row={auctionID=1000+slot,itemKey={itemID=10},quantity=slot,status=0,
                bidder='PRIVATE',itemLink='PRIVATE',bidAmount=SECRET,buyoutAmount=SECRET}
            if mode=='duplicate' then row.auctionID=1 end
            if mode=='maximum' then row.auctionID=9007199254740991-slot;row.quantity=4294967295;row.status=4294967295 end
            if mode=='changing_row' and reads>2 then row.quantity=99 end
            if mode=='restricted' then row.itemKey.itemID=SECRET end
            if mode=='bad_quantity' then row.quantity=0 end
            if mode=='bad_id' then row.auctionID=9007199254740992 end
            if mode=='missing' then return nil end
            if mode=='error' then error('PRIVATE') end
            if mode=='replace_api' then C_AuctionHouse.GetOwnedAuctionInfo=function() return row end end
            if mode=='close' then AuctionHouseFrame.IsShown=function() return false end end
            return row
        end
        C_AuctionHouse.QueryOwnedAuctions=forbidden;C_AuctionHouse.SendBrowseQuery=forbidden
        C_AuctionHouse.SendSearchQuery=forbidden;C_AuctionHouse.CancelAuction=forbidden
        C_AuctionHouse.PostItem=forbidden;C_AuctionHouse.PlaceBid=forbidden
        C_AuctionHouse.GetOwnedAuctions=forbidden;C_AuctionHouse.GetBidInfo=forbidden
        C_AuctionHouse.RequestOwnedAuctionBidderInfo=forbidden
    "#).exec()?;
    let namespace: mlua::Table = lua.globals().get("AO")?;
    lua.load(include_str!("../addon/Collectors.lua")).call::<()>(("AzerothOperator",namespace.clone()))?;
    lua.load(include_str!("../addon/Core.lua")).call::<()>(("AzerothOperator",namespace))?;
    lua.load("AO.Capture()").exec()?;
    let exported: String = lua.globals().get("AzerothOperatorExport")?;
    let snapshot=decode(format!("AzerothOperatorExport = \"{exported}\"").as_bytes())?;
    assert!(!serde_json::to_string(&snapshot)?.contains("PRIVATE"));
    assert_eq!(lua.globals().get::<u32>("actions")?,0);
    Ok((snapshot,lua))
}

#[test]
fn stable_owned_cache_roundtrips_without_private_fields_or_queries() -> Result<(),Box<dyn std::error::Error>> {
    let (snapshot,lua)=capture("normal")?;
    assert_eq!(snapshot.sections["owned_auctions"],json!({
        "coverage":"loaded_player_owned_auction_cache","entries":{
            "1":{"auction_id":1001,"item_id":10,"quantity":1,"reported_status":0},
            "2":{"auction_id":1002,"item_id":10,"quantity":2,"reported_status":0}}}));
    assert_eq!(lua.globals().get::<u32>("reads")?,4);
    assert_eq!(capture("empty")?.0.sections["owned_auctions"]["entries"],json!({}));
    assert_eq!(capture("maximum")?.0.sections["owned_auctions"]["entries"].as_object().unwrap().len(),256);
    Ok(())
}

#[test]
fn hidden_changing_restricted_or_invalid_cache_is_unavailable() -> Result<(),Box<dyn std::error::Error>> {
    for mode in ["hidden","over_budget","changing_count","changing_row","restricted","bad_quantity",
        "bad_id","missing","error","duplicate","replace_api","close"] {
        let (snapshot,_)=capture(mode)?;
        assert!(!snapshot.sections.contains_key("owned_auctions"),"{mode}");
        assert_eq!(snapshot.unavailable["owned_auctions"],"api_unavailable_or_failed");
    }
    Ok(())
}
