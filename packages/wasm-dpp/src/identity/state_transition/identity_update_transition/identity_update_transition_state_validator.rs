use std::sync::Arc;

use crate::state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper};
use crate::validation::ValidationResultWasm;
use crate::{IdentityUpdateTransitionWasm, StateTransitionExecutionContextWasm};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use dpp::identity::state_transition::identity_update_transition::identity_update_transition::IdentityUpdateTransition;
use dpp::identity::state_transition::identity_update_transition::validate_identity_update_transition_state::IdentityUpdateTransitionStateValidator;
use dpp::identity::state_transition::identity_update_transition::validate_public_keys::IdentityUpdatePublicKeysValidator;
use crate::errors::from_dpp_err;

#[wasm_bindgen(js_name=IdentityUpdateTransitionStateValidator)]
pub struct IdentityUpdateTransitionStateValidatorWasm(
    IdentityUpdateTransitionStateValidator<
        IdentityUpdatePublicKeysValidator,
        ExternalStateRepositoryLikeWrapper,
    >,
);

#[wasm_bindgen(js_class=IdentityUpdateTransitionStateValidator)]
impl IdentityUpdateTransitionStateValidatorWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        state_repository: ExternalStateRepositoryLike,
    ) -> Result<IdentityUpdateTransitionStateValidatorWasm, JsValue> {
        let state_repository = ExternalStateRepositoryLikeWrapper::new(state_repository);

        let public_keys_validator = IdentityUpdatePublicKeysValidator {};

        let validator = IdentityUpdateTransitionStateValidator::new(
            Arc::new(state_repository),
            Arc::new(public_keys_validator),
        );

        Ok(Self(validator))
    }

    #[wasm_bindgen]
    pub async fn validate(
        &self,
        state_transition: &IdentityUpdateTransitionWasm,
        execution_context: &StateTransitionExecutionContextWasm,
    ) -> Result<ValidationResultWasm, JsValue> {
        let state_transition: IdentityUpdateTransition = state_transition.to_owned().into();

        let validation_result = self
            .0
            .validate(&state_transition, &execution_context.to_owned().into())
            .await
            .map_err(|e| from_dpp_err(e.into()))?;

        Ok(validation_result.map(|_| JsValue::undefined()).into())
    }
}
