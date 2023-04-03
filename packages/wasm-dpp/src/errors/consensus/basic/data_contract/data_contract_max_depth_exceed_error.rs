use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DataContractMaxDepthExceedError)]
pub struct DataContractMaxDepthExceedErrorWasm {
    max_depth: usize,
    schema_depth: usize,
    code: u32,
}

impl DataContractMaxDepthExceedErrorWasm {
    pub fn new(schema_depth: usize, max_depth: usize, code: u32) -> Self {
        DataContractMaxDepthExceedErrorWasm {
            max_depth,
            schema_depth,
            code,
        }
    }
}

#[wasm_bindgen(js_class=DataContractMaxDepthError)]
impl DataContractMaxDepthExceedErrorWasm {
    #[wasm_bindgen(js_name=getMaxDepth)]
    pub fn get_max_depth(&self) -> usize {
        self.max_depth
    }

    #[wasm_bindgen(js_name=getSchemaDepth)]
    pub fn get_schema_depth(&self) -> usize {
        self.schema_depth
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
