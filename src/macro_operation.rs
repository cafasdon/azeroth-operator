//! Compile explicit macro operations as bounded data for player-applied addon import.
use crate::{Family, Snapshot};
use anyhow::{Result, ensure};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

#[derive(Clone, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MacroFields {
    pub name: String,
    pub icon: u32,
    pub body: String,
}
#[derive(Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Account,
    Character,
}
impl Scope {
    fn text(&self) -> &'static str {
        match self {
            Self::Account => "account",
            Self::Character => "character",
        }
    }
}
#[derive(Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MacroRequest {
    Create {
        target_scope: Scope,
        desired: MacroFields,
    },
    Edit {
        source_index: u32,
        desired: MacroFields,
    },
    Delete {
        source_index: u32,
    },
    Move {
        source_index: u32,
        target_scope: Scope,
    },
}
fn hex(text: &str) -> String {
    if text.is_empty() {
        "-".into()
    } else {
        text.as_bytes().iter().map(|b| format!("{b:02x}")).collect()
    }
}
fn validate(fields: &MacroFields) -> Result<()> {
    ensure!(
        !fields.name.is_empty()
            && fields.name.len() <= 16
            && !fields.name.chars().any(char::is_control),
        "Invalid macro name"
    );
    ensure!(
        fields.body.len() <= 255 && !fields.body.contains('\0') && fields.icon > 0,
        "Invalid macro body/icon"
    );
    Ok(())
}
pub fn compile(snapshot: &Snapshot, snapshot_id: &str, request: &MacroRequest) -> Result<Value> {
    snapshot.validate()?;
    ensure!(
        snapshot.client.family == Family::Retail,
        "Unsupported client family"
    );
    ensure!(
        snapshot_id.len() == 64
            && snapshot_id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "Invalid snapshot ID"
    );
    let macros = snapshot
        .sections
        .get("macros")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow::anyhow!("Macro observation unavailable"))?;
    let (kind, index, target, desired) = match request {
        MacroRequest::Create {
            target_scope,
            desired,
        } => ("create", None, Some(target_scope), Some(desired)),
        MacroRequest::Edit {
            source_index,
            desired,
        } => ("edit", Some(source_index), None, Some(desired)),
        MacroRequest::Delete { source_index } => ("delete", Some(source_index), None, None),
        MacroRequest::Move {
            source_index,
            target_scope,
        } => ("move", Some(source_index), Some(target_scope), None),
    };
    let source = index
        .map(|i| {
            macros
                .get(&i.to_string())
                .ok_or_else(|| anyhow::anyhow!("Macro not observed"))
        })
        .transpose()?;
    let expected = source
        .map(|s| {
            serde_json::from_value::<MacroFields>(
                json!({"name":s["name"],"icon":s["icon"],"body":s["body"]}),
            )
        })
        .transpose()?;
    if let Some(value) = &expected {
        validate(value)?;
    }
    if let Some(value) = desired {
        validate(value)?;
    }
    let source_scope = source.and_then(|s| s["scope"].as_str()).unwrap_or("-");
    if kind == "move" {
        ensure!(
            source_scope != target.unwrap().text(),
            "Already in target scope"
        );
    }
    let mut fields = vec![
        "AO-MACRO-1".into(),
        String::new(),
        snapshot_id.into(),
        "retail".into(),
        hex(&snapshot.client.variant),
        hex(&snapshot.client.build),
        snapshot.client.interface.to_string(),
        hex(&snapshot.character.name),
        hex(&snapshot.character.realm),
        kind.into(),
        source_scope.into(),
        target.map(|s| s.text()).unwrap_or("-").into(),
    ];
    for value in [expected.as_ref(), desired] {
        if let Some(value) = value {
            fields.extend([hex(&value.name), value.icon.to_string(), hex(&value.body)]);
        } else {
            fields.extend(["-".into(), "-".into(), "-".into()]);
        }
    }
    fields[1] = format!("{:x}", Sha256::digest(fields.join("|").as_bytes()));
    let code = fields.join("|");
    ensure!(code.len() <= 8192, "Operation too large");
    Ok(
        json!({"operation_id":fields[1],"snapshot_id":snapshot_id,"client":snapshot.client,"character":snapshot.character,
        "kind":kind,"source_scope":source_scope,"target_scope":target,"expected":expected,"desired":desired,
        "import_code":code,"requires_player_apply":true,"game_execution":false,"live_qualified":false,
        "action_bar_references":"not_relocated"}),
    )
}
