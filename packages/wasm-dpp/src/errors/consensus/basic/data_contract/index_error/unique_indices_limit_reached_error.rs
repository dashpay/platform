use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=UniqueIndicesLimitReachedError)]
pub struct UniqueIndicesLimitReachedErrorWasm {
    document_type: String,
    index_limit: usize,
    code: u32,
}

impl UniqueIndicesLimitReachedErrorWasm {
    pub fn new(document_type: String, index_limit: usize, code: u32) -> Self {
        UniqueIndicesLimitReachedErrorWasm {
            document_type,
            index_limit,
            code,
        }
    }
}

#[wasm_bindgen(js_class=UniqueIndicesLimitReachedError)]
impl UniqueIndicesLimitReachedErrorWasm {
    #[wasm_bindgen(js_name=getDocumentType)]
    pub fn get_document_type(&self) -> String {
        self.document_type.clone()
    }

    #[wasm_bindgen(js_name=getIndexLimit)]
    pub fn get_index_limit(&self) -> usize {
        self.index_limit
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}

