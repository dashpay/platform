use dpp::consensus::ConsensusError;

use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

use crate::{
    buffer::Buffer, errors::consensus::consensus_error::from_consensus_error_ref,
    utils::consensus_errors_from_buffers,
};

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
    #[wasm_bindgen(constructor)]
    pub fn new_wasm(
        error_buffers: Vec<Buffer>,
        raw_state_transition: JsValue,
    ) -> Result<InvalidStateTransitionErrorWasm, JsValue> {
        let consensus_errors = consensus_errors_from_buffers(error_buffers)?;

        Ok(InvalidStateTransitionErrorWasm::new(
            consensus_errors,
            raw_state_transition,
        ))
    }

    #[wasm_bindgen(js_name = getErrors)]
    pub fn get_errors(&self) -> Vec<JsValue> {
        self.errors.iter().map(from_consensus_error_ref).collect()
    }

    #[wasm_bindgen(js_name = getRawStateTransition)]
    pub fn get_raw_state_transition(&self) -> JsValue {
        self.raw_state_transition.clone()
    }

    // #[wasm_bindgen(js_name = valueOf)]
    // pub fn value_of(&self) -> Result<JsValue, JsError> {
    //     let errors = serde_json::to_value(&self.errors)
    //         .map_err(|_| JsError::new("Can't serialize consensus errors to json"))?;
    //     let ser = serde_wasm_bindgen::Serializer::json_compatible();
    //     errors
    //         .serialize(&ser)
    //         .map_err(|_| JsError::new("Can't serialize consensus errors to json"))
    // }
}
