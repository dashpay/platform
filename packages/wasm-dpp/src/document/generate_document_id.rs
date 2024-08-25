use dpp::document::Document;
use wasm_bindgen::prelude::*;

use crate::identifier::{identifier_from_js_value, IdentifierWrapper};

#[wasm_bindgen(js_name=generateDocumentId)]
pub fn generate_document_id_wasm(
    contract_id: &JsValue,
    owner_id: &JsValue,
    document_type: String,
    entropy: Vec<u8>,
) -> Result<IdentifierWrapper, JsValue> {
    let contract_id = identifier_from_js_value(contract_id)?;
    let owner_id = identifier_from_js_value(owner_id)?;
    let id = Document::generate_document_id_v0(&contract_id, &owner_id, &document_type, &entropy);
    Ok(id.into())
}
