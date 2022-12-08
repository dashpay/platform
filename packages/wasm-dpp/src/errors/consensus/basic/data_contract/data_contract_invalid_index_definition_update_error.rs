

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DataContractInvalidIndexDefinitionUpdateError)]
pub struct DataContractInvalidIndexDefinitionUpdateErrorWasm {
    document_type: String,
    index_name: String,
    code: u32,
}

impl DataContractInvalidIndexDefinitionUpdateErrorWasm {
    pub fn new(document_type: String, index_name: String, code: u32) -> Self {
        DataContractInvalidIndexDefinitionUpdateErrorWasm {
            document_type,
            index_name,
            code,
        }
    }
}

#[wasm_bindgen(js_class=DataContractInvalidIndexDefinitionUpdateError)]
impl DataContractInvalidIndexDefinitionUpdateErrorWasm {
    #[wasm_bindgen(js_name=getDocumentType)]
    pub fn get_document_type(&self) -> String {
        self.document_type.clone()
    }

    #[wasm_bindgen(js_name=getIndexName)]
    pub fn get_index_name(&self) -> String {
        self.index_name.clone()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
