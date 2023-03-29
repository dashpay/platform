use thiserror::Error;
use wasm_bindgen::prelude::*;

use crate::identifier::IdentifierWrapper;

#[wasm_bindgen]
#[derive(Error, Debug)]
#[error("Invalid Document Type")]
pub struct InvalidDocumentTypeInDataContractError {
    // we have to store it as JsValue as the errors of 'class' Consensus are of different types
    doc_type: String,
    data_contract_id: IdentifierWrapper,
}

#[wasm_bindgen]
impl InvalidDocumentTypeInDataContractError {
    #[wasm_bindgen(constructor)]
    pub fn new(doc_type: String, data_contract_id: IdentifierWrapper) -> Self {
        InvalidDocumentTypeInDataContractError {
            doc_type,
            data_contract_id,
        }
    }
    #[wasm_bindgen(js_name = "getType")]
    pub fn get_doc_type(&self) -> String {
        self.doc_type.clone()
    }

    #[wasm_bindgen(js_name = "getDataContractId")]
    pub fn get_data_contract_id(&self) -> IdentifierWrapper {
        self.data_contract_id.clone()
    }
}
