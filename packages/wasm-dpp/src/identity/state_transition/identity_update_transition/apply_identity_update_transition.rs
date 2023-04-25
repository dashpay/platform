use dpp::identity::state_transition::identity_update_transition::apply_identity_update_transition::apply_identity_update_transition;
use wasm_bindgen::prelude::*;

use crate::{
    state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper},
    IdentityUpdateTransitionWasm, StateTransitionExecutionContextWasm,
};

#[wasm_bindgen(js_name=applyIdentityUpdateTransition)]
pub async fn apply_identity_update_transition_wasm(
    state_repository: ExternalStateRepositoryLike,
    state_transition: &IdentityUpdateTransitionWasm,
    execution_context: &StateTransitionExecutionContextWasm,
) -> Result<(), JsValue> {
    let wrapped_state_repository = ExternalStateRepositoryLikeWrapper::new(state_repository);

    apply_identity_update_transition(
        &wrapped_state_repository,
        &state_transition.to_owned().into(),
        &execution_context.to_owned().into(),
    )
    .await
    .map_err(|e| JsValue::from_str(&e.to_string()))?;

    Ok(())
}
