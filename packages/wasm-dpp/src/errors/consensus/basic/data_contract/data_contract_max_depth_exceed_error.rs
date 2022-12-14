use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=DataContractMaxDepthError)]
pub struct DataContractMaxDepthErrorWasm {
    depth: usize,
    code: u32,
}

impl DataContractMaxDepthErrorWasm {
    pub fn new(depth: usize, code: u32) -> Self {
        DataContractMaxDepthErrorWasm { depth, code }
    }
}

#[wasm_bindgen(js_class=DataContractMaxDepthError)]
impl DataContractMaxDepthErrorWasm {
    #[wasm_bindgen(js_name=getDepth)]
    pub fn get_expected_version(&self) -> usize {
        self.depth
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
