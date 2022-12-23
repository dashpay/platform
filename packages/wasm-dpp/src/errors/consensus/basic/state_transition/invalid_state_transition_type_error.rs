use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidStateTransitionTypeError)]
pub struct InvalidStateTransitionTypeErrorWasm {
    transition_type: u8,
    code: u32,
}

impl InvalidStateTransitionTypeErrorWasm {
    pub fn new(transition_type: u8, code: u32) -> Self {
        InvalidStateTransitionTypeErrorWasm {
            transition_type,
            code,
        }
    }
}

#[wasm_bindgen(js_class=InvalidStateTransitionTypeError)]
impl InvalidStateTransitionTypeErrorWasm {
    #[wasm_bindgen(js_name=getTransitionType)]
    pub fn get_transition_type(&self) -> u8 {
        self.transition_type
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        self.code
    }
}
