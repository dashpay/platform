use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name=BatchedTransition)]
#[derive(Debug, Clone)]
pub struct BatchedTransitionWasm(BatchedTransition);

impl From<BatchedTransition> for BatchedTransitionWasm {
    fn from(t: BatchedTransition) -> Self {
        BatchedTransitionWasm(t)
    }
}

impl From<BatchedTransitionWasm> for BatchedTransition {
    fn from(t: BatchedTransitionWasm) -> Self {
        t.0
    }
}
