use crate::bls_adapter::{BlsAdapter, JsBlsAdapter};
use crate::errors::from_dpp_err;
use crate::state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper};
use crate::utils::ToSerdeJSONExt;
use crate::validation::ValidationResultWasm;
use dpp::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use dpp::state_transition::{StateTransitionConvert, StateTransitionFacade, StateTransitionLike};
use serde_json::Value;
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

#[wasm_bindgen(js_class = StateTransitionFacade)]
impl StateTransitionFacadeWasm {
    #[wasm_bindgen(js_name = validateBasic)]
    pub async fn validate_basic(
        &self,
        raw_state_transition: JsValue,
    ) -> Result<ValidationResultWasm, JsValue> {
        let state_transition_json;
        let execution_context;

        if let Ok(state_transition) =
            super::super::conversion::create_state_transition_from_wasm_instance(
                &raw_state_transition,
            )
        {
            execution_context = state_transition.get_execution_context().to_owned();
            state_transition_json =
                super::super::conversion::state_transition_wasm_to_object(&raw_state_transition)?
                    .with_serde_to_json_value()?;
        } else {
            state_transition_json = raw_state_transition.with_serde_to_json_value()?;
            execution_context = StateTransitionExecutionContext::default();
        }

        let validation_result = self
            .0
            .validate_basic(&state_transition_json, &execution_context)
            .await
            .map_err(from_dpp_err)?;

        Ok(validation_result.map(|_| JsValue::undefined()).into())
    }

    #[wasm_bindgen(js_name = validateSignature)]
    pub async fn validate_signature(
        &self,
        raw_state_transition: JsValue,
    ) -> Result<ValidationResultWasm, JsValue> {
        let state_transition =
            super::super::conversion::create_state_transition_from_wasm_instance(
                &raw_state_transition,
            )?;

        let validation_result = self
            .0
            .validate_signature(state_transition)
            .await
            .map_err(from_dpp_err)?;

        web_sys::console::log_1(&"Validated st".into());

        Ok(validation_result.map(|_| JsValue::undefined()).into())
    }
}
