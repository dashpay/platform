use crate::buffer::Buffer;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::state::document::duplicate_unique_index_error::DuplicateUniqueIndexError;
use dpp::consensus::ConsensusError;

use js_sys::JsString;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DuplicateUniqueIndexError)]
pub struct DuplicateUniqueIndexErrorWasm {
    inner: DuplicateUniqueIndexError,
}

impl From<&DuplicateUniqueIndexError> for DuplicateUniqueIndexErrorWasm {
    fn from(e: &DuplicateUniqueIndexError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=DuplicateUniqueIndexError)]
impl DuplicateUniqueIndexErrorWasm {
    #[wasm_bindgen(js_name=getDocumentId)]
    pub fn document_id(&self) -> Buffer {
        Buffer::from_bytes(self.inner.document_id().as_bytes())
    }

    #[wasm_bindgen(js_name=getDuplicatingProperties)]
    pub fn duplicating_properties(&self) -> js_sys::Array {
        self.inner
            .duplicating_properties()
            .iter()
            .map(|string| JsString::from(string.as_ref()))
            .collect()
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
