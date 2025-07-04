use crate::sdk::WasmSdk;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};

#[wasm_bindgen]
pub async fn get_status(_sdk: &WasmSdk) -> Result<JsValue, JsError> {
    Err(JsError::new("getStatus is not yet implemented in the WASM SDK"))
}

#[wasm_bindgen]
pub async fn get_total_credits_in_platform(_sdk: &WasmSdk) -> Result<u64, JsError> {
    Err(JsError::new("getTotalCreditsInPlatform is not yet implemented in the WASM SDK"))
}

#[wasm_bindgen]
pub async fn get_current_quorums_info(_sdk: &WasmSdk) -> Result<JsValue, JsError> {
    Err(JsError::new("getCurrentQuorumsInfo is not yet implemented in the WASM SDK"))
}

#[wasm_bindgen]
pub async fn get_prefunded_specialized_balance(_sdk: &WasmSdk, _identity_id: &str) -> Result<JsValue, JsError> {
    Err(JsError::new("getPrefundedSpecializedBalance is not yet implemented in the WASM SDK"))
}

#[wasm_bindgen]
pub async fn get_path_elements(_sdk: &WasmSdk, _keys: Vec<String>) -> Result<JsValue, JsError> {
    Err(JsError::new("getPathElements is not yet implemented in the WASM SDK"))
}

#[wasm_bindgen]
pub async fn wait_for_state_transition_result(_sdk: &WasmSdk, _state_transition_hash: &str) -> Result<JsValue, JsError> {
    Err(JsError::new("waitForStateTransitionResult is not yet implemented in the WASM SDK"))
}