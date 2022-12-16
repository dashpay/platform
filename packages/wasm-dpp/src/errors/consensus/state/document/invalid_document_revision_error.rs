use crate::buffer::Buffer;
use dpp::{identifier::Identifier, prelude::Revision};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidDocumentRevisionError)]
pub struct InvalidDocumentRevisionErrorWasm {
    document_id: Identifier,
    current_revision: Revision,
    code: u32,
}

#[wasm_bindgen(js_class=InvalidDocumentRevisionError)]
impl InvalidDocumentRevisionErrorWasm {
    #[wasm_bindgen(js_name=getDocumentId)]
    pub fn document_id(&self) -> Buffer {
        Buffer::from_bytes(self.document_id.as_bytes())
    }

    #[wasm_bindgen(js_name=getCurrentRevision)]
    pub fn current_revision(&self) -> u64 {
        self.current_revision
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}

impl InvalidDocumentRevisionErrorWasm {
    pub fn new(document_id: Identifier, current_revision: Revision, code: u32) -> Self {
        Self {
            document_id,
            current_revision,
            code,
        }
    }
}
