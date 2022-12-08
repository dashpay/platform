use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=MissingStateTransitionTypeError)]
pub struct MissingStateTransitionTypeErrorWasm {
    code: u32,
}

impl MissingStateTransitionTypeErrorWasm {
    pub fn new(code: u32) -> Self {
        MissingStateTransitionTypeErrorWasm { code }
    }
}

#[wasm_bindgen(js_class=MissingStateTransitionTypeError)]
impl MissingStateTransitionTypeErrorWasm {
    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
