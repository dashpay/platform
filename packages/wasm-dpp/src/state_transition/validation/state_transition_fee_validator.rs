use crate::state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper};
use dpp::state_transition::state_transition_validation::validate_state_transition_fee::StateTransitionFeeValidator;
use std::sync::Arc;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

use crate::state_transition::conversion::create_state_transition_from_wasm_instance;
use crate::utils::WithJsError;
use crate::validation::ValidationResultWasm;
use crate::StateTransitionExecutionContextWasm;

#[wasm_bindgen(js_name=StateTransitionFeeValidator)]
pub struct StateTransitionFeeValidatorWasm(
    StateTransitionFeeValidator<ExternalStateRepositoryLikeWrapper>,
);

#[wasm_bindgen(js_class=StateTransitionFeeValidator)]
impl StateTransitionFeeValidatorWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(state_repository: ExternalStateRepositoryLike) -> Self {
        Self(StateTransitionFeeValidator::new(Arc::new(
            ExternalStateRepositoryLikeWrapper::new(state_repository),
        )))
    }

    #[wasm_bindgen]
    pub async fn validate(
        &self,
        state_transition: &JsValue,
        execution_context: &StateTransitionExecutionContextWasm,
    ) -> Result<ValidationResultWasm, JsValue> {
        let st = create_state_transition_from_wasm_instance(state_transition)?;
        Ok(self
            .0
            .validate(&st, &execution_context.to_owned().into())
            .await
            .map(|v| v.map(|_| JsValue::undefined()))
            .with_js_error()?
            .into())
    }
}
