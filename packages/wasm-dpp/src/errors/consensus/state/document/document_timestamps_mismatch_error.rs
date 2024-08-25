use crate::buffer::Buffer;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::state::document::document_timestamps_mismatch_error::DocumentTimestampsMismatchError;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DocumentTimestampsMismatchError)]
pub struct DocumentTimestampsMismatchErrorWasm {
    inner: DocumentTimestampsMismatchError,
}

impl From<&DocumentTimestampsMismatchError> for DocumentTimestampsMismatchErrorWasm {
    fn from(e: &DocumentTimestampsMismatchError) -> Self {
        Self { inner: e.clone() }
    }
}
#[wasm_bindgen(js_class=DocumentTimestampsMismatchError)]
impl DocumentTimestampsMismatchErrorWasm {
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
