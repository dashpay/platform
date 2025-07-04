use crate::sdk::WasmSdk;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};

#[wasm_bindgen]
pub async fn get_epochs_info(
    _sdk: &WasmSdk,
    _start_epoch: Option<u16>,
    _count: u32,
    _ascending: Option<bool>,
) -> Result<JsValue, JsError> {
    Err(JsError::new("getEpochsInfo is not yet implemented in the WASM SDK"))
}

#[wasm_bindgen]
pub async fn get_finalized_epoch_infos(
    _sdk: &WasmSdk,
    _start_epoch: Option<u16>,
    _count: Option<u32>,
    _ascending: Option<bool>,
) -> Result<JsValue, JsError> {
    Err(JsError::new("getFinalizedEpochInfos is not yet implemented in the WASM SDK"))
}

#[wasm_bindgen]
pub async fn get_evonodes_proposed_epoch_blocks_by_ids(
    _sdk: &WasmSdk,
    _epoch: u32,
    _ids: Vec<String>,
) -> Result<JsValue, JsError> {
    Err(JsError::new("getEvonodesProposedEpochBlocksByIds is not yet implemented in the WASM SDK"))
}

#[wasm_bindgen]
pub async fn get_evonodes_proposed_epoch_blocks_by_range(
    _sdk: &WasmSdk,
    _epoch: u32,
    _limit: Option<u32>,
    _start_after: Option<String>,
    _order_ascending: Option<bool>,
) -> Result<JsValue, JsError> {
    Err(JsError::new("getEvonodesProposedEpochBlocksByRange is not yet implemented in the WASM SDK"))
}