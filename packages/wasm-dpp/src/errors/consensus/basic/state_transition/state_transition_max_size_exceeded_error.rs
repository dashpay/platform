use dpp::consensus::basic::state_transition::StateTransitionMaxSizeExceededError;
use dpp::consensus::codes::ErrorWithCode;
use dpp::consensus::ConsensusError;

use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name=StateTransitionMaxSizeExceededError)]
pub struct StateTransitionMaxSizeExceededErrorWasm {
    inner: StateTransitionMaxSizeExceededError,
}

impl From<&StateTransitionMaxSizeExceededError> for StateTransitionMaxSizeExceededErrorWasm {
    fn from(e: &StateTransitionMaxSizeExceededError) -> Self {
        Self { inner: e.clone() }
    }
}

#[wasm_bindgen(js_class=StateTransitionMaxSizeExceededError)]
impl StateTransitionMaxSizeExceededErrorWasm {
    #[wasm_bindgen(js_name=getActualSizeBytes)]
    pub fn get_actual_size_bytes(&self) -> u64 {
        self.inner.actual_size_bytes()
    }

    #[wasm_bindgen(js_name=getMaxSizeBytes)]
    pub fn get_max_size_bytes(&self) -> u64 {
        self.inner.max_size_bytes()
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
