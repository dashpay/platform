use crate::sdk::WasmSdk;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};

#[wasm_bindgen]
pub async fn get_documents(
    _sdk: &WasmSdk,
    _data_contract_id: &str,
    _document_type: &str,
    _where_clause: Option<String>,
    _order_by: Option<String>,
    _limit: Option<u32>,
    _start_after: Option<String>,
    _start_at: Option<String>,
) -> Result<JsValue, JsError> {
    Err(JsError::new("getDocuments with where/orderBy clauses is not yet fully implemented in the WASM SDK"))
}