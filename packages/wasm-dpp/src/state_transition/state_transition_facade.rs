use crate::bls_adapter::{BlsAdapter, JsBlsAdapter};
use crate::errors::from_dpp_err;
use crate::state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper};
use dpp::state_transition::StateTransitionFacade;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[derive(Clone)]
#[wasm_bindgen(js_name = StateTransitionFacade)]
pub struct StateTransitionFacadeWasm(
    StateTransitionFacade<ExternalStateRepositoryLikeWrapper, BlsAdapter>,
);

impl StateTransitionFacadeWasm {
    pub fn new(
        state_repository: ExternalStateRepositoryLike,
        bls_adapter: JsBlsAdapter,
    ) -> Result<StateTransitionFacadeWasm, JsValue> {
        let state_repository_wrapper =
            Arc::new(ExternalStateRepositoryLikeWrapper::new(state_repository));

        let adapter = BlsAdapter(bls_adapter);

        let state_transition_facade =
            StateTransitionFacade::new(state_repository_wrapper, adapter).map_err(from_dpp_err)?;

        Ok(StateTransitionFacadeWasm(state_transition_facade))
    }
}
