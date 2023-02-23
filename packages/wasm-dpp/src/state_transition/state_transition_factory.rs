use dpp::state_transition::StateTransitionFactory;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::state_repository::ExternalStateRepositoryLikeWrapper;

#[wasm_bindgen(js_name = StateTransitionFactory)]
pub struct StateTransitionFactoryWasm(StateTransitionFactory<ExternalStateRepositoryLikeWrapper>);
