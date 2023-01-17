use js_sys::JsString;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InconsistentCompoundIndexDataError)]
pub struct InconsistentCompoundIndexDataErrorWasm {
    index_properties: Vec<String>,
    document_type: String,
    code: u32,
}

impl InconsistentCompoundIndexDataErrorWasm {
    pub fn new(index_properties: Vec<String>, document_type: String, code: u32) -> Self {
        InconsistentCompoundIndexDataErrorWasm {
            index_properties,
            document_type,
            code,
        }
    }
}

#[wasm_bindgen(js_class=InconsistentCompoundIndexDataError)]
impl InconsistentCompoundIndexDataErrorWasm {
    #[wasm_bindgen(js_name=getIndexedProperties)]
    pub fn get_indexed_properties(&self) -> js_sys::Array {
        self.index_properties
            .iter()
            .map(|string| JsString::from(string.as_ref()))
            .collect()
    }

    #[wasm_bindgen(js_name=getDocumentType)]
    pub fn get_document_type(&self) -> String {
        self.document_type.clone()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
