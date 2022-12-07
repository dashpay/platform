use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidStateTransitionTypeError)]
pub struct StateTransitionMaxSizeExceededErrorWasm {
    actual_size_kbytes: usize,
    max_size_kbytes: usize,
    code: u32,
}

impl StateTransitionMaxSizeExceededErrorWasm {
    pub fn new(actual_size_kbytes: usize, max_size_kbytes: usize, code: u32) -> Self {
        StateTransitionMaxSizeExceededErrorWasm {
            actual_size_kbytes,
            max_size_kbytes,
            code,
        }
    }
}

#[wasm_bindgen(js_class=InvalidStateTransitionTypeError)]
impl StateTransitionMaxSizeExceededErrorWasm {
    #[wasm_bindgen(js_name=getActualSizeKBytes)]
    pub fn get_actual_size_kbytes(&self) -> usize {
        self.actual_size_kbytes
    }

    #[wasm_bindgen(js_name=getMaxSizeKBytes)]
    pub fn get_max_size_kbytes(&self) -> usize {
        self.max_size_kbytes
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
