use dpp::identity::state_transition::asset_lock_proof::AssetLockTransactionOutputFetcher;
use dpp::identity::state_transition::identity_topup_transition::ApplyIdentityTopUpTransition;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

use crate::{
    state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper},
    IdentityTopUpTransitionWasm, StateTransitionExecutionContextWasm,
};

#[wasm_bindgen(js_name=applyIdentityTopUpTransition)]
pub async fn apply_identity_topup_transition_wasm(
    state_repository: ExternalStateRepositoryLike,
    state_transition: &IdentityTopUpTransitionWasm,
    execution_context: &StateTransitionExecutionContextWasm,
) -> Result<(), JsValue> {
    let wrapped_state_repository =
        Arc::new(ExternalStateRepositoryLikeWrapper::new(state_repository));

    let tx_output_fetcher =
        AssetLockTransactionOutputFetcher::new(wrapped_state_repository.clone());

    let instance =
        ApplyIdentityTopUpTransition::new(wrapped_state_repository, Arc::new(tx_output_fetcher));

    instance
        .apply(
            &state_transition.to_owned().into(),
            &execution_context.to_owned().into(),
        )
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    Ok(())
}
