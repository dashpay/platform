use crate::buffer::Buffer;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::state::document::invalid_document_revision_error::InvalidDocumentRevisionError;
use dpp::consensus::ConsensusError;
use dpp::prelude::Revision;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidDocumentRevisionError)]
pub struct InvalidDocumentRevisionErrorWasm {
    inner: InvalidDocumentRevisionError,
}

impl From<&InvalidDocumentRevisionError> for InvalidDocumentRevisionErrorWasm {
    fn from(e: &InvalidDocumentRevisionError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InvalidDocumentRevisionError)]
impl InvalidDocumentRevisionErrorWasm {
    #[wasm_bindgen(js_name=getDocumentId)]
    pub fn document_id(&self) -> Buffer {
        Buffer::from_bytes(self.inner.document_id().as_bytes())
    }

    #[wasm_bindgen(js_name=getPreviousRevision)]
    pub fn previous_revision(&self) -> Option<Revision> {
        *self.inner.previous_revision()
    }

    #[wasm_bindgen(js_name=getDesiredRevision)]
    pub fn desired_revision(&self) -> Revision {
        *self.inner.desired_revision()
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
