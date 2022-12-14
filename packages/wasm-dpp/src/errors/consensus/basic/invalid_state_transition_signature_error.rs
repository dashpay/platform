use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidStateTransitionSignatureError)]
pub struct InvalidStateTransitionSignatureErrorWasm {
    code: u32,
}

impl InvalidStateTransitionSignatureErrorWasm {
    pub fn new(code: u32) -> Self {
        InvalidStateTransitionSignatureErrorWasm { code }
    }
}

#[wasm_bindgen(js_class=InvalidStateTransitionSignatureError)]
impl InvalidStateTransitionSignatureErrorWasm {
    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
