use lazy_static::lazy_static;
use serde_json::Value as JsonValue;

pub mod data_contract;
pub mod identity;

lazy_static! {
    pub static ref BASE_TRANSITION_SCHEMA: JsonValue = serde_json::from_str(include_str!(
        "./document/v0/stateTransition/documentTransition/base.json"
    ))
    .unwrap();
    pub static ref CREATE_TRANSITION_SCHEMA: JsonValue = serde_json::from_str(include_str!(
        "./document/v0/stateTransition/documentTransition/create.json"
    ))
    .unwrap();
    pub static ref REPLACE_TRANSITION_SCHEMA: JsonValue = serde_json::from_str(include_str!(
        "./document/v0/stateTransition/documentTransition/replace.json"
    ))
    .unwrap();
    pub static ref DOCUMENTS_BATCH_TRANSITIONS_SCHEMA: JsonValue = serde_json::from_str(
        include_str!("./document/v0/stateTransition/documentsBatch.json")
    )
    .unwrap();
}
