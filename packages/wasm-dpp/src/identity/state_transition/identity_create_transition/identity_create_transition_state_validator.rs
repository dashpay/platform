use crate::errors::from_dpp_err;
use crate::state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper};
use crate::validation::ValidationResultWasm;
use crate::IdentityCreateTransitionWasm;
use dpp::identity::state_transition::identity_create_transition::validation::state::validate_identity_create_transition_state;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name=IdentityCreateTransitionStateValidator)]
pub struct IdentityCreateTransitionStateValidator {
    state_repository: ExternalStateRepositoryLikeWrapper,
}

#[wasm_bindgen(js_class=IdentityCreateTransitionStateValidator)]
impl IdentityCreateTransitionStateValidator {
    #[wasm_bindgen(constructor)]
    pub fn new(state_repository: ExternalStateRepositoryLike) -> Self {
        Self {
            state_repository: ExternalStateRepositoryLikeWrapper::new(state_repository),
        }
    }

    #[wasm_bindgen(js_name = validate)]
    pub async fn validate(
        &self,
        state_transition: &IdentityCreateTransitionWasm,
    ) -> Result<ValidationResultWasm, JsValue> {
        let validation_result = validate_identity_create_transition_state(
            &self.state_repository,
            state_transition.to_owned().clone().into(),
        )
        .await
        .map_err(|e| from_dpp_err(e.into()))?;

        Ok(validation_result.map(|_| JsValue::undefined()).into())
    }
}
