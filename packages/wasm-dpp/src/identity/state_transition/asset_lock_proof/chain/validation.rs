use dpp::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use dpp::identity::state_transition::asset_lock_proof::{
    AssetLockTransactionValidator, ChainAssetLockProofStructureValidator, PublicKeyHash,
};
use dpp::identity::state_transition::identity_create_transition::validation::state::validate_identity_create_transition_state;
use dpp::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use dpp::validation::ValidationResult;
use dpp::ProtocolError;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use web_sys::console::log_1;

use crate::{
    errors::from_dpp_err,
    state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper},
    validation::ValidationResultWasm,
    AssetLockProofWasm, ChainAssetLockProofWasm, IdentityCreateTransitionWasm,
    StateTransitionExecutionContextWasm,
};

#[wasm_bindgen(js_name=validateChainAssetLockProofStructure)]
pub async fn validate_chain_asset_lock_proof_structure(
    state_repository: ExternalStateRepositoryLike, // TODO: test with wrapper?
    asset_lock_proof: &ChainAssetLockProofWasm,    // TODO: should be AssetLockProofWasm?
    execution_context: StateTransitionExecutionContextWasm,
) -> Result<ValidationResultWasm, JsValue> {
    let state_repository_wrapper =
        Arc::new(ExternalStateRepositoryLikeWrapper::new(state_repository));

    let asset_lock_proof: ChainAssetLockProof = asset_lock_proof.to_owned().into();
    let asset_lock_proof_json =
        serde_json::to_value(asset_lock_proof.clone()).map_err(|e| from_dpp_err(e.into()))?;

    let tx_validator = Arc::new(AssetLockTransactionValidator::new(
        state_repository_wrapper.clone(),
    ));

    let validator =
        ChainAssetLockProofStructureValidator::new(state_repository_wrapper, tx_validator)
            .map_err(|e| from_dpp_err(ProtocolError::Generic(e.to_string())))?;

    let validation_result = validator
        .validate(&asset_lock_proof_json, &execution_context.into())
        .await
        .map(|value| {
            let test: ValidationResultWasm = value.into();
            test
        })
        .map_err(|e| from_dpp_err(e.into()))?;
    Ok(validation_result.into())
}
