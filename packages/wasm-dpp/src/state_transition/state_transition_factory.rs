use dpp::state_transition::StateTransitionFactory;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::{bls_adapter::BlsAdapter, state_repository::ExternalStateRepositoryLikeWrapper};

#[wasm_bindgen(js_name = StateTransitionFactory)]
pub struct StateTransitionFactoryWasm(
    StateTransitionFactory<ExternalStateRepositoryLikeWrapper, BlsAdapter>,
);
