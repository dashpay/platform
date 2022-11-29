use crate::buffer::Buffer;
use dpp::identifier::Identifier;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DocumentAlreadyPresentError)]
pub struct DocumentAlreadyPresentErrorWasm {
    document_id: Identifier,
    code: u32,
}

#[wasm_bindgen(js_class=DocumentAlreadyPresentError)]
impl DocumentAlreadyPresentErrorWasm {
    #[wasm_bindgen(js_name=getDocumentId)]
    pub fn document_id(&self) -> Buffer {
        Buffer::from_bytes(self.document_id.as_bytes())
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}

impl DocumentAlreadyPresentErrorWasm {
    pub fn new(document_id: Identifier, code: u32) -> Self {
        Self { document_id, code }
    }
}
