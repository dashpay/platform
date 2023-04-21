use dpp::identity::state_transition::identity_create_transition::ApplyIdentityCreateTransition;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

use crate::{
    state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper},
    IdentityCreateTransitionWasm, StateTransitionExecutionContextWasm,
};

#[wasm_bindgen(js_name=applyIdentityCreateTransition)]
pub async fn apply_identity_create_transition_wasm(
    state_repository: ExternalStateRepositoryLike,
    state_transition: &IdentityCreateTransitionWasm,
    execution_context: &StateTransitionExecutionContextWasm,
) -> Result<(), JsValue> {
    let wrapped_state_repository =
        Arc::new(ExternalStateRepositoryLikeWrapper::new(state_repository));

    let instance = ApplyIdentityCreateTransition::new(wrapped_state_repository);

    instance
        .apply_identity_create_transition(
            &state_transition.to_owned().into(),
            &execution_context.to_owned().into(),
        )
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    Ok(())
}
