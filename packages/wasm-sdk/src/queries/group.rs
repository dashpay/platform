use crate::sdk::WasmSdk;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};

#[wasm_bindgen]
pub async fn get_group_info(_sdk: &WasmSdk, _group_contract_id: &str) -> Result<JsValue, JsError> {
    Err(JsError::new("getGroupInfo is not yet implemented in the WASM SDK"))
}

#[wasm_bindgen]
pub async fn get_group_members(
    _sdk: &WasmSdk,
    _group_contract_id: &str,
    _member_ids: Option<Vec<String>>,
    _start_at: Option<String>,
    _limit: Option<u32>,
) -> Result<JsValue, JsError> {
    Err(JsError::new("getGroupMembers is not yet implemented in the WASM SDK"))
}

#[wasm_bindgen]
pub async fn get_identity_groups(
    _sdk: &WasmSdk,
    _identity_id: &str,
    _member_data_contracts: Option<Vec<String>>,
    _owner_data_contracts: Option<Vec<String>>,
    _moderator_data_contracts: Option<Vec<String>>,
) -> Result<JsValue, JsError> {
    Err(JsError::new("getIdentityGroups is not yet implemented in the WASM SDK"))
}

#[wasm_bindgen]
pub async fn get_groups_data_contracts(_sdk: &WasmSdk, _data_contract_ids: Vec<String>) -> Result<JsValue, JsError> {
    Err(JsError::new("getGroupsDataContracts is not yet implemented in the WASM SDK"))
}