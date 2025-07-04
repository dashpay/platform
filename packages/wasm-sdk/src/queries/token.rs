use crate::sdk::WasmSdk;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};

#[wasm_bindgen]
pub async fn get_identities_token_balances(
    _sdk: &WasmSdk,
    _identity_ids: Vec<String>,
    _token_id: &str,
) -> Result<JsValue, JsError> {
    Err(JsError::new("getIdentitiesTokenBalances is not yet implemented in the WASM SDK"))
}

#[wasm_bindgen]
pub async fn get_identity_token_infos(
    _sdk: &WasmSdk,
    _identity_id: &str,
    _token_ids: Option<Vec<String>>,
    _with_purchase_info: Option<bool>,
    _limit: Option<u32>,
    _offset: Option<u32>,
) -> Result<JsValue, JsError> {
    Err(JsError::new("getIdentityTokenInfos is not yet implemented in the WASM SDK"))
}

#[wasm_bindgen]
pub async fn get_identities_token_infos(
    _sdk: &WasmSdk,
    _identity_ids: Vec<String>,
    _token_id: &str,
    _with_purchase_info: Option<bool>,
) -> Result<JsValue, JsError> {
    Err(JsError::new("getIdentitiesTokenInfos is not yet implemented in the WASM SDK"))
}

#[wasm_bindgen]
pub async fn get_token_statuses(_sdk: &WasmSdk, _token_ids: Vec<String>) -> Result<JsValue, JsError> {
    Err(JsError::new("getTokenStatuses is not yet implemented in the WASM SDK"))
}

#[wasm_bindgen]
pub async fn get_token_direct_purchase_prices(_sdk: &WasmSdk, _token_ids: Vec<String>) -> Result<JsValue, JsError> {
    Err(JsError::new("getTokenDirectPurchasePrices is not yet implemented in the WASM SDK"))
}

#[wasm_bindgen]
pub async fn get_token_contract_info(_sdk: &WasmSdk, _data_contract_id: &str) -> Result<JsValue, JsError> {
    Err(JsError::new("getTokenContractInfo is not yet implemented in the WASM SDK"))
}

#[wasm_bindgen]
pub async fn get_token_perpetual_distribution_last_claim(
    _sdk: &WasmSdk,
    _identity_id: &str,
    _distribution_id: &str,
) -> Result<JsValue, JsError> {
    Err(JsError::new("getTokenPerpetualDistributionLastClaim is not yet implemented in the WASM SDK"))
}

#[wasm_bindgen]
pub async fn get_token_total_supply(_sdk: &WasmSdk, _token_id: &str) -> Result<JsValue, JsError> {
    Err(JsError::new("getTokenTotalSupply is not yet implemented in the WASM SDK"))
}