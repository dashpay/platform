use crate::errors::consensus::consensus_error::from_consensus_error_ref;
use dpp::consensus::ConsensusError;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name=InvalidIdentityError)]
pub struct InvalidIdentityError {
    errors: Vec<ConsensusError>,
    raw_identity: JsValue,
}

impl InvalidIdentityError {
    pub fn new(errors: Vec<ConsensusError>, raw_identity: JsValue) -> Self {
        InvalidIdentityError {
            errors,
            raw_identity,
        }
    }
}

#[wasm_bindgen(js_class=InvalidIdentityError)]
impl InvalidIdentityError {
    #[wasm_bindgen(js_name=getErrors)]
    pub fn get_errors(&self) -> Vec<JsValue> {
        self.errors.iter().map(from_consensus_error_ref).collect()
    }

    #[wasm_bindgen(js_name=getRawIdentity)]
    pub fn get_raw_identity(&self) -> JsValue {
        self.raw_identity.clone()
    }
}
