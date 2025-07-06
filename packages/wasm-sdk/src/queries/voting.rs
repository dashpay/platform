use crate::sdk::WasmSdk;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};

#[wasm_bindgen]
pub async fn get_contested_resources(
    _sdk: &WasmSdk,
    _document_type_name: &str,
    _data_contract_id: &str,
    _index_name: &str,
    _result_type: &str,
    _allow_include_locked_and_abstaining_vote_tally: Option<bool>,
    _start_at_value: Option<Vec<u8>>,
    _limit: Option<u32>,
    _offset: Option<u32>,
    _order_ascending: Option<bool>,
) -> Result<JsValue, JsError> {
    Err(JsError::new("getContestedResources is not yet implemented in the WASM SDK. The voting/contested resource queries require additional SDK support that is not yet available in the WASM build."))
}


#[wasm_bindgen]
pub async fn get_contested_resource_vote_state(
    _sdk: &WasmSdk,
    _data_contract_id: &str,
    _document_type_name: &str,
    _index_name: &str,
    _result_type: &str,
    _allow_include_locked_and_abstaining_vote_tally: Option<bool>,
    _start_at_identifier_info: Option<String>,
    _count: Option<u32>,
    _order_ascending: Option<bool>,
) -> Result<JsValue, JsError> {
    Err(JsError::new("getContestedResourceVoteState is not yet implemented in the WASM SDK. The voting/contested resource queries require additional SDK support that is not yet available in the WASM build."))
}


#[wasm_bindgen]
pub async fn get_contested_resource_voters_for_identity(
    _sdk: &WasmSdk,
    _data_contract_id: &str,
    _document_type_name: &str,
    _index_name: &str,
    _contestant_id: &str,
    _start_at_identifier_info: Option<String>,
    _count: Option<u32>,
    _order_ascending: Option<bool>,
) -> Result<JsValue, JsError> {
    Err(JsError::new("getContestedResourceVotersForIdentity is not yet implemented in the WASM SDK. The voting/contested resource queries require additional SDK support that is not yet available in the WASM build."))
}


#[wasm_bindgen]
pub async fn get_contested_resource_identity_votes(
    _sdk: &WasmSdk,
    _identity_id: &str,
    _limit: Option<u32>,
    _offset: Option<u32>,
    _order_ascending: Option<bool>,
) -> Result<JsValue, JsError> {
    Err(JsError::new("getContestedResourceIdentityVotes is not yet implemented in the WASM SDK. The voting/contested resource queries require additional SDK support that is not yet available in the WASM build."))
}


#[wasm_bindgen]
pub async fn get_vote_polls_by_end_date(
    _sdk: &WasmSdk,
    _start_time_ms: Option<u64>,
    _end_time_ms: Option<u64>,
    _limit: Option<u32>,
    _offset: Option<u32>,
    _order_ascending: Option<bool>,
) -> Result<JsValue, JsError> {
    Err(JsError::new("getVotePollsByEndDate is not yet implemented in the WASM SDK. The voting/contested resource queries require additional SDK support that is not yet available in the WASM build."))
}