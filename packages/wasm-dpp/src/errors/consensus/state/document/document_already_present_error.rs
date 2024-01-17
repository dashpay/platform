use crate::buffer::Buffer;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::state::document::document_already_present_error::DocumentAlreadyPresentError;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DocumentAlreadyPresentError)]
pub struct DocumentAlreadyPresentErrorWasm {
    inner: DocumentAlreadyPresentError,
}

impl From<&DocumentAlreadyPresentError> for DocumentAlreadyPresentErrorWasm {
    fn from(e: &DocumentAlreadyPresentError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=DocumentAlreadyPresentError)]
impl DocumentAlreadyPresentErrorWasm {
    #[wasm_bindgen(js_name=getDocumentId)]
    pub fn document_id(&self) -> Buffer {
        Buffer::from_bytes(self.inner.document_id().as_bytes())
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
