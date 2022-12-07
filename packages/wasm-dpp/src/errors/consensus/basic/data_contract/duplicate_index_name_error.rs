use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DuplicateIndexNameError)]
pub struct DuplicateIndexNameErrorWasm {
    document_type: String,
    duplicate_index_name: String,
    code: u32,
}

impl DuplicateIndexNameErrorWasm {
    pub fn new(document_type: String, duplicate_index_name: String, code: u32) -> Self {
        DuplicateIndexNameErrorWasm {
            document_type,
            duplicate_index_name,
            code,
        }
    }
}

#[wasm_bindgen(js_class=DuplicateIndexNameError)]
impl DuplicateIndexNameErrorWasm {
    #[wasm_bindgen(js_name=getDocumentType)]
    pub fn get_document_type(&self) -> String {
        self.document_type
    }

    #[wasm_bindgen(js_name=getDuplicateIndexName)]
    pub fn get_duplicate_index_name(&self) -> String {
        self.duplicate_index_name
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
