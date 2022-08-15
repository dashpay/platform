//! This module contains data structures that are left to be implemented

use wasm_bindgen::prelude::*;

use dpp::document::document_transition::DocumentTransition;
use dpp::errors::consensus::ConsensusError as DPPConsensusError;

#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct DocumentTransitionWasm(DocumentTransition);

impl DocumentTransitionWasm {
    pub fn get_action(&self) -> String {
        unimplemented!()
    }
}

impl From<DocumentTransition> for DocumentTransitionWasm {
    fn from(v: DocumentTransition) -> Self {
        DocumentTransitionWasm(v)
    }
}

#[derive(Debug)]
pub struct ConsensusError {}

pub fn from_consensus_to_js_error(_: DPPConsensusError) -> JsValue {
    unimplemented!()
}
