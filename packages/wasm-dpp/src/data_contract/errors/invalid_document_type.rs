use thiserror::Error;
use wasm_bindgen::prelude::*;

use crate::DataContractWasm;

#[wasm_bindgen]
#[derive(Error, Debug)]
#[error("Invalid Document Type")]
pub struct InvalidDocumentTypeError {
    // we have to store it as JsValue as the errors of 'class' Consensus are of different types
    doc_type: String,
    data_contract: DataContractWasm,
}

#[wasm_bindgen]
impl InvalidDocumentTypeError {
    #[wasm_bindgen(constructor)]
    pub fn new(doc_type: String, data_contract: DataContractWasm) -> Self {
        InvalidDocumentTypeError {
            doc_type,
            data_contract,
        }
    }
    #[wasm_bindgen(js_name = "getDocType")]
    pub fn get_doc_type(&self) -> String {
	self.doc_type.clone()
    }

    #[wasm_bindgen(js_name = "getDataContract")]
    pub fn get_data_contract(&self) -> DataContractWasm {
        self.data_contract.clone()
    }
}
