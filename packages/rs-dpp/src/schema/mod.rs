use lazy_static::lazy_static;
// use serde_json::Value as JsonValue;

pub mod identity;

lazy_static! {
    pub static ref BASE_TRANSITION_SCHEMA: serde_json::Value = serde_json::from_str(include_str!(
        "./document/v0/stateTransition/documentTransition/base.json"
    ))
    .unwrap();
    pub static ref CREATE_TRANSITION_SCHEMA: serde_json::Value = serde_json::from_str(include_str!(
        "./document/v0/stateTransition/documentTransition/create.json"
    ))
    .unwrap();
    pub static ref REPLACE_TRANSITION_SCHEMA: serde_json::Value = serde_json::from_str(include_str!(
        "./document/v0/stateTransition/documentTransition/replace.json"
    ))
    .unwrap();
    pub static ref DOCUMENTS_BATCH_TRANSITIONS_SCHEMA: serde_json::Value = serde_json::from_str(
        include_str!("./document/v0/stateTransition/documentsBatch.json")
    )
    .unwrap();
}
