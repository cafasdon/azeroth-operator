//! Version-one inventory payload validation. Other sections remain independently staged.
use anyhow::{Result, ensure};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Item {
    item_id: u32,
    link: Option<String>,
    count: Option<u32>,
}

fn index(key: &str, minimum: u32, maximum: u32) -> Result<()> {
    let value: u32 = key.parse()?;
    ensure!(
        value >= minimum && value <= maximum && value.to_string() == key,
        "Invalid slot index"
    );
    Ok(())
}

fn items(value: &Value, equipment: bool) -> Result<()> {
    let slots = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("Slots must be an object"))?;
    for (slot, value) in slots {
        index(slot, 1, if equipment { 19 } else { 1024 })?;
        let item: Item = serde_json::from_value(value.clone())?;
        ensure!(item.item_id > 0, "Invalid item ID");
        ensure!(
            item.link.as_ref().is_none_or(|s| s.len() <= 4096),
            "Item link too long"
        );
        ensure!(
            !equipment || item.count.is_none(),
            "Equipment cannot carry a stack count"
        );
        ensure!(
            equipment || item.count.is_some_and(|n| n > 0),
            "Missing or invalid stack count"
        );
    }
    Ok(())
}

pub fn validate(name: &str, value: &Value) -> Result<()> {
    match name {
        "equipment" => items(value, true)?,
        "bags" => {
            let bags = value
                .as_object()
                .ok_or_else(|| anyhow::anyhow!("Bags must be an object"))?;
            for (bag, slots) in bags {
                index(bag, 0, 64)?;
                items(slots, false)?;
            }
        }
        _ => (),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn rejects_ambiguous_or_malformed_inventory() {
        for value in [
            json!([]),
            json!({"01":{"item_id":1}}),
            json!({"20":{"item_id":1}}),
            json!({"1":{"item_id":0}}),
            json!({"1":{"item_id":1,"extra":true}}),
        ] {
            assert!(validate("equipment", &value).is_err());
        }
        for value in [
            json!({"0":[]}),
            json!({"0":{"1":{"item_id":1}}}),
            json!({"0":{"1":{"item_id":1,"count":-1}}}),
        ] {
            assert!(validate("bags", &value).is_err());
        }
        validate("equipment", &json!({"16":{"item_id":123,"link":null}})).unwrap();
        validate("bags", &json!({"0":{"1":{"item_id":123,"count":2}}})).unwrap();
    }
}
