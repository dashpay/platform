use dpp::consensus::basic::document::InvalidDocumentTypeError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

use crate::buffer::Buffer;

#[wasm_bindgen(js_name=InvalidDocumentTypeError)]
pub struct InvalidDocumentTypeErrorWasm {
    inner: InvalidDocumentTypeError,
}

impl From<&InvalidDocumentTypeError> for InvalidDocumentTypeErrorWasm {
    fn from(e: &InvalidDocumentTypeError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InvalidDocumentTypeError)]
impl InvalidDocumentTypeErrorWasm {
    #[wasm_bindgen(js_name=getType)]
    pub fn get_document_type(&self) -> String {
        self.inner.document_type()
    }

    #[wasm_bindgen(js_name=getDataContractId)]
    pub fn get_data_contract_id(&self) -> Buffer {
        Buffer::from_bytes(self.inner.data_contract_id().as_bytes())
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }
}
