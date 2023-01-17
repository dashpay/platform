use dpp::identity::state_transition::asset_lock_proof::{
    AssetLockTransactionValidator, InstantAssetLockProofStructureValidator,
};
use dpp::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use dpp::ProtocolError;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

use crate::buffer::Buffer;
use crate::utils::ToSerdeJSONExt;
use crate::{
    errors::from_dpp_err,
    state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper},
    validation::ValidationResultWasm,
    StateTransitionExecutionContextWasm,
};

#[wasm_bindgen(js_name=validateInstantAssetLockProofStructure)]
pub async fn validate_instant_asset_lock_proof_structure(
    state_repository: ExternalStateRepositoryLike, // TODO: test with wrapper?
    raw_asset_lock_proof: JsValue,
    execution_context: &StateTransitionExecutionContextWasm,
) -> Result<ValidationResultWasm, JsValue> {
    let state_repository_wrapper =
        Arc::new(ExternalStateRepositoryLikeWrapper::new(state_repository));

    let asset_lock_proof_json = raw_asset_lock_proof.with_serde_to_json_value()?;

    let tx_validator = Arc::new(AssetLockTransactionValidator::new(
        state_repository_wrapper.clone(),
    ));

    let validator =
        InstantAssetLockProofStructureValidator::new(state_repository_wrapper, tx_validator)
            .map_err(|e| from_dpp_err(ProtocolError::Generic(e.to_string())))?;

    let context: &StateTransitionExecutionContext = execution_context.into();
    let validation_result = validator
        .validate(&asset_lock_proof_json, context)
        .await
        .map_err(|e| from_dpp_err(e.into()))?;

    Ok(validation_result
        .map(|item| JsValue::from(Buffer::from_bytes(&item)))
        .into())
}
