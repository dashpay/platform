use crate::errors::from_dpp_err;
use crate::state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper};
use crate::validation::ValidationResultWasm;
use crate::{IdentityTopUpTransitionWasm, StateTransitionExecutionContextWasm};
use dpp::identity::state_transition::identity_topup_transition::validation::state::validate_identity_topup_transition_state;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name=IdentityTopUpTransitionStateValidator)]
pub struct IdentityTopUpTransitionStateValidator {
    state_repository: ExternalStateRepositoryLikeWrapper,
}

#[wasm_bindgen(js_class=IdentityTopUpTransitionStateValidator)]
impl IdentityTopUpTransitionStateValidator {
    #[wasm_bindgen(constructor)]
    pub fn new(state_repository: ExternalStateRepositoryLike) -> Self {
        Self {
            state_repository: ExternalStateRepositoryLikeWrapper::new(state_repository),
        }
    }

    #[wasm_bindgen]
    pub async fn validate(
        &self,
        state_transition: &IdentityTopUpTransitionWasm,
        execution_context: &StateTransitionExecutionContextWasm,
    ) -> Result<ValidationResultWasm, JsValue> {
        let validation_result = validate_identity_topup_transition_state(
            &self.state_repository,
            &state_transition.to_owned().into(),
            &execution_context.to_owned().into(),
        )
        .await
        .map_err(|e| from_dpp_err(e.into()))?;

        Ok(validation_result.map(|_| JsValue::undefined()).into())
    }
}
