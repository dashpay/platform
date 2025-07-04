use crate::dpp::DataContractWasm;
use crate::sdk::WasmSdk;
use dash_sdk::platform::{DataContract, Fetch, Identifier};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};

#[wasm_bindgen]
pub async fn data_contract_fetch(
    sdk: &WasmSdk,
    base58_id: &str,
) -> Result<DataContractWasm, JsError> {
    let id = Identifier::from_string(
        base58_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    DataContract::fetch_by_identifier(sdk, id)
        .await?
        .ok_or_else(|| JsError::new("Data contract not found"))
        .map(Into::into)
}

#[wasm_bindgen]
pub async fn get_data_contract_history(
    _sdk: &WasmSdk,
    _id: &str,
    _limit: Option<u32>,
    _offset: Option<u32>,
    _start_at_ms: Option<u64>,
) -> Result<JsValue, JsError> {
    Err(JsError::new("getDataContractHistory is not yet implemented in the WASM SDK"))
}

#[wasm_bindgen]
pub async fn get_data_contracts(_sdk: &WasmSdk, _ids: Vec<String>) -> Result<JsValue, JsError> {
    Err(JsError::new("getDataContracts is not yet implemented in the WASM SDK"))
}