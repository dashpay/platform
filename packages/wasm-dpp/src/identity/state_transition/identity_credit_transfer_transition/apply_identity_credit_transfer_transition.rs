use dpp::identity::state_transition::identity_credit_transfer_transition::apply_identity_credit_transfer::ApplyIdentityCreditTransferTransition;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

use crate::identity::state_transition::identity_credit_transfer_transition::transition::IdentityCreditTransferTransitionWasm;
use crate::{
    state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper},
    StateTransitionExecutionContextWasm,
};

#[wasm_bindgen(js_name=applyIdentityCreditTransferTransition)]
pub async fn apply_identity_credit_transfer_transition_wasm(
    state_repository: ExternalStateRepositoryLike,
    state_transition: &IdentityCreditTransferTransitionWasm,
    execution_context: &StateTransitionExecutionContextWasm,
) -> Result<(), JsValue> {
    let wrapped_state_repository =
        Arc::new(ExternalStateRepositoryLikeWrapper::new(state_repository));

    let instance = ApplyIdentityCreditTransferTransition::new(wrapped_state_repository);

    instance
        .apply(
            &state_transition.to_owned().into(),
            &execution_context.to_owned().into(),
        )
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    Ok(())
}
