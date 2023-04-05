use dpp::consensus::ConsensusError;
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

use crate::errors::consensus::consensus_error::from_consensus_error_ref;

#[wasm_bindgen(js_name = InvalidStateTransitionError)]
pub struct InvalidStateTransitionErrorWasm {
    errors: Vec<ConsensusError>,
    raw_state_transition: JsValue,
}

impl InvalidStateTransitionErrorWasm {
    pub fn new(errors: Vec<ConsensusError>, raw_state_transition: JsValue) -> Self {
        Self {
            errors,
            raw_state_transition,
        }
    }
}

#[wasm_bindgen(js_class = InvalidStateTransitionError)]
impl InvalidStateTransitionErrorWasm {
    #[wasm_bindgen(js_name = getErrors)]
    pub fn get_errors(&self) -> Vec<JsValue> {
        self.errors.iter().map(from_consensus_error_ref).collect()
    }

    #[wasm_bindgen(js_name = getRawStateTransition)]
    pub fn get_raw_state_transition(&self) -> JsValue {
        self.raw_state_transition.clone()
    }
}
