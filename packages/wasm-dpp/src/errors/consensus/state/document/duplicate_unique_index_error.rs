use crate::buffer::Buffer;
use dpp::identifier::Identifier;
use js_sys::JsString;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DuplicateUniqueIndexError)]
pub struct DuplicateUniqueIndexErrorWasm {
    document_id: Identifier,
    duplicating_properties: Vec<String>,
    code: u32,
}

#[wasm_bindgen(js_class=DuplicateUniqueIndexError)]
impl DuplicateUniqueIndexErrorWasm {
    #[wasm_bindgen(js_name=getDocumentId)]
    pub fn document_id(&self) -> Buffer {
        Buffer::from_bytes(self.document_id.as_bytes())
    }

    #[wasm_bindgen(js_name=getDuplicatingProperties)]
    pub fn duplicating_properties(&self) -> js_sys::Array {
        self.duplicating_properties
            .iter()
            .map(|string| JsString::from(string.as_ref()))
            .collect()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}

impl DuplicateUniqueIndexErrorWasm {
    pub fn new(document_id: Identifier, duplicating_properties: Vec<String>, code: u32) -> Self {
        Self {
            document_id,
            duplicating_properties,
            code,
        }
    }
}
