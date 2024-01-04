use crate::buffer::Buffer;
use dpp::consensus::basic::document::InvalidDocumentTransitionIdError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidDocumentTransitionIdError)]
pub struct InvalidDocumentTransitionIdErrorWasm {
    inner: InvalidDocumentTransitionIdError,
}

impl From<&InvalidDocumentTransitionIdError> for InvalidDocumentTransitionIdErrorWasm {
    fn from(e: &InvalidDocumentTransitionIdError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InvalidDocumentTransitionIdError)]
impl InvalidDocumentTransitionIdErrorWasm {
    #[wasm_bindgen(js_name=getExpectedId)]
    pub fn get_expected_id(&self) -> Buffer {
        Buffer::from_bytes(self.inner.expected_id().as_bytes())
    }

    #[wasm_bindgen(js_name=getInvalidId)]
    pub fn get_invalid_id(&self) -> Buffer {
        Buffer::from_bytes(self.inner.invalid_id().as_bytes())
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
