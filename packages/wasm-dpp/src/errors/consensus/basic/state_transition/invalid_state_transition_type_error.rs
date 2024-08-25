use dpp::consensus::basic::state_transition::InvalidStateTransitionTypeError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=InvalidStateTransitionTypeError)]
pub struct InvalidStateTransitionTypeErrorWasm {
    inner: InvalidStateTransitionTypeError,
}

impl From<&InvalidStateTransitionTypeError> for InvalidStateTransitionTypeErrorWasm {
    fn from(e: &InvalidStateTransitionTypeError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=InvalidStateTransitionTypeError)]
impl InvalidStateTransitionTypeErrorWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(transition_type: u8) -> Self {
        Self {
            inner: InvalidStateTransitionTypeError::new(transition_type),
        }
    }

    #[wasm_bindgen(js_name=getType)]
    pub fn get_type(&self) -> u8 {
        self.inner.transition_type()
    }

    #[wasm_bindgen(js_name=getCode)]
    pub fn get_code(&self) -> u32 {
        ConsensusError::from(self.inner.clone()).code()
    }

    #[wasm_bindgen(getter)]
    pub fn message(&self) -> String {
        self.inner.to_string()
    }
}
