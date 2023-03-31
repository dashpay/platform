use dpp::util::json_schema::Index;
use wasm_bindgen::prelude::*;

use crate::IndexDefinitionWasm;

#[wasm_bindgen(js_name=InvalidCompoundIndexError)]
pub struct InvalidCompoundIndexErrorWasm {
    document_type: String,
    index_name: String,
    code: u32,
}

impl InvalidCompoundIndexErrorWasm {
    pub fn new(document_type: String, index_name: String, code: u32) -> Self {
        InvalidCompoundIndexErrorWasm {
            document_type,
            index_name,
            code,
        }
    }
}

#[wasm_bindgen(js_class=InvalidCompoundIndexError)]
impl InvalidCompoundIndexErrorWasm {
    #[wasm_bindgen(js_name=getDocumentType)]
    pub fn get_document_type(&self) -> String {
        self.document_type.clone()
    }

    #[wasm_bindgen(js_name=getIndexName)]
    pub fn get_index_name(&self) -> JsValue {
        IndexDefinitionWasm::from(self.index_name.clone()).into()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
