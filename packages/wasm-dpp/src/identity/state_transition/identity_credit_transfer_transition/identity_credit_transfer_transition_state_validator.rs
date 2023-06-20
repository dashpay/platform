use crate::errors::from_dpp_err;
use crate::execution::validation::ValidationResultWasm;
use crate::state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper};
use crate::{IdentityCreditTransferTransitionWasm, StateTransitionExecutionContextWasm};
use dpp::identity::state_transition::identity_credit_transfer_transition::validation::state::IdentityCreditTransferTransitionStateValidator;
use dpp::identity::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name=IdentityCreditTransferTransitionStateValidator)]
pub struct IdentityCreditTransferTransitionStateValidatorWasm(
    IdentityCreditTransferTransitionStateValidator<ExternalStateRepositoryLikeWrapper>,
);

#[wasm_bindgen(js_class=IdentityCreditTransferTransitionStateValidator)]
impl IdentityCreditTransferTransitionStateValidatorWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        state_repository: ExternalStateRepositoryLike,
    ) -> Result<IdentityCreditTransferTransitionStateValidatorWasm, JsValue> {
        let state_repository = ExternalStateRepositoryLikeWrapper::new(state_repository);

        let validator = IdentityCreditTransferTransitionStateValidator::new(state_repository);

        Ok(Self(validator))
    }

    #[wasm_bindgen]
    pub async fn validate(
        &self,
        state_transition: &IdentityCreditTransferTransitionWasm,
        execution_context: &StateTransitionExecutionContextWasm,
    ) -> Result<ValidationResultWasm, JsValue> {
        let state_transition: IdentityCreditTransferTransition = state_transition.to_owned().into();

        let validation_result = self
            .0
            .validate(&state_transition, &execution_context.to_owned().into())
            .await
            .map_err(|e| from_dpp_err(e))?;

        Ok(validation_result.map(|_| JsValue::undefined()).into())
    }
}
