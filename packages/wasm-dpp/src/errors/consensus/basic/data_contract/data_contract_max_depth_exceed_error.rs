use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DataContractMaxDepthExceedError)]
pub struct DataContractMaxDepthExceedErrorWasm {
    depth: usize,
    code: u32,
}

impl DataContractMaxDepthExceedErrorWasm {
    pub fn new(depth: usize, code: u32) -> Self {
        DataContractMaxDepthExceedErrorWasm { depth, code }
    }
}

#[wasm_bindgen(js_class=DataContractMaxDepthError)]
impl DataContractMaxDepthExceedErrorWasm {
    #[wasm_bindgen(js_name=getDepth)]
    pub fn get_expected_version(&self) -> usize {
        self.depth
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
