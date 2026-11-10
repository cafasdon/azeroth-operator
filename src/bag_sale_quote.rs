//! Conditional saved sale-value arithmetic for explicitly selected bag locations.
use crate::{Family, Snapshot};
use anyhow::{Result, ensure};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::BTreeSet, num::NonZeroU32};

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    #[schemars(range(min=0, max=64))]
    pub container: u8,
    #[schemars(range(min=1, max=1024))]
    pub slot: u16,
    pub item_id: NonZeroU32,
    pub quantity: NonZeroU32,
}

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Quote {
    #[schemars(length(min=1, max=20))]
    pub selections: Vec<Selection>,
    #[serde(default)]
    #[schemars(range(min=0, max=9_999_999_999_999u64))]
    pub reserve_copper: u64,
}

impl Quote {
    pub fn validate(&self) -> Result<()> {
        ensure!((1..=20).contains(&self.selections.len()) && self.reserve_copper<=9_999_999_999_999,
            "Invalid bag sale quote bounds");
        let mut locations = BTreeSet::new();
        for row in &self.selections {
            ensure!(row.container<=64 && (1..=1024).contains(&row.slot)
                && locations.insert((row.container,row.slot)), "Invalid bag sale location");
        }
        Ok(())
    }
}

pub fn review(snapshot: &Snapshot, quote: &Quote) -> Result<Value> {
    quote.validate()?;
    ensure!(snapshot.client.family==Family::Forever, "Expected Forever");
    let bags = snapshot.sections.get("bags").and_then(Value::as_object)
        .ok_or_else(||anyhow::anyhow!("Bags unavailable"))?;
    let mut rows = Vec::new();
    let (mut unknown_prices,mut overflow_rows) = (0,0);
    let mut subtotal = Some(0u64);
    let mut selections: Vec<_> = quote.selections.iter().collect();
    selections.sort_by_key(|row|(row.container,row.slot));
    for selected in selections {
        let row = bags.get(&selected.container.to_string())
            .and_then(|bag|bag.get(selected.slot.to_string()))
            .ok_or_else(||anyhow::anyhow!("Bag location unavailable"))?;
        let observed = row["count"].as_u64().ok_or_else(||anyhow::anyhow!("Stack count unavailable"))?;
        let quantity = u64::from(selected.quantity.get());
        ensure!(row["item_id"].as_u64()==Some(u64::from(selected.item_id.get())) && quantity<=observed,
            "Bag selection does not match saved stack");
        let price = row["sell_price_copper"].as_u64();
        let amount = price.and_then(|price|price.checked_mul(quantity));
        let status = match (price,amount) {
            (None,_) => {unknown_prices+=1; "price_not_recorded"},
            (Some(_),None) => {overflow_rows+=1; "arithmetic_overflow"},
            (_,Some(amount)) => {subtotal=subtotal.and_then(|sum|sum.checked_add(amount)); "quoted"},
        };
        rows.push(json!({"container":selected.container,"slot":selected.slot,"item_id":selected.item_id.get(),
            "observed_stack_quantity":observed,"selected_quantity":quantity,"unit_price_copper":price,
            "quoted_proceeds_copper":amount,"status":status}));
    }
    let complete = unknown_prices==0 && overflow_rows==0 && subtotal.is_some();
    let total = subtotal.filter(|_|complete);
    let wallet = snapshot.sections.get("wallet").and_then(|wallet|wallet["copper"].as_u64());
    let projected = total.zip(wallet).and_then(|(amount,wallet)|wallet.checked_add(amount));
    let wallet_overflow = total.zip(wallet).is_some() && projected.is_none();
    Ok(json!({"basis":"explicit_saved_bag_locations_at_captured_unit_sell_prices",
        "captured_at":snapshot.captured_at,"rows":rows,"price_quote_complete":complete,
        "unpriced_selection_count":unknown_prices,"overflow_selection_count":overflow_rows,
        "known_proceeds_subtotal_copper":subtotal,"total_proceeds_copper":total,
        "proceeds_arithmetic_overflow":overflow_rows>0 || subtotal.is_none(),
        "captured_wallet_copper":wallet,"conditional_wallet_copper_after_sale":projected,
        "wallet_projection_arithmetic_overflow":wallet_overflow,"reserve_copper":quote.reserve_copper,
        "conditional_copper_above_reserve":projected.map(|amount|amount.saturating_sub(quote.reserve_copper)),
        "conditional_reserve_shortfall_copper":projected.map(|amount|quote.reserve_copper.saturating_sub(amount)),
        "wallet_projection_is_hypothetical":true,"sale_recommendation":false,
        "current_funds_credited":false,"bank_or_mail_credit":false,
        "current_locations_verified":false,"native_sale_eligibility_verified":false,
        "game_execution":false,"live":false}))
}
