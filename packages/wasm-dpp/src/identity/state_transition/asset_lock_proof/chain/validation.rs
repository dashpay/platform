use dpp::identity::state_transition::asset_lock_proof::{
    AssetLockTransactionValidator, ChainAssetLockProofStructureValidator,
};
use dpp::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use dpp::ProtocolError;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

use crate::buffer::Buffer;
use crate::{
    errors::{from_dpp_err, RustConversionError},
    state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper},
    validation::ValidationResultWasm,
    with_js_error, StateTransitionExecutionContextWasm,
};

#[skip_serializing_none]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawChainAssetLockProof {
    #[serde(rename = "type")]
    lock_type: Option<u8>,
    core_chain_locked_height: Option<u32>,
    out_point: Option<Vec<u8>>,
}

#[wasm_bindgen(js_name=validateChainAssetLockProofStructure)]
pub async fn validate_chain_asset_lock_proof_structure(
    state_repository: ExternalStateRepositoryLike, // TODO: test with wrapper?
    raw_asset_lock_proof: JsValue,
    execution_context: &StateTransitionExecutionContextWasm,
) -> Result<ValidationResultWasm, JsValue> {
    let state_repository_wrapper =
        Arc::new(ExternalStateRepositoryLikeWrapper::new(state_repository));

    let parsed_asset_lock_proof: RawChainAssetLockProof =
        with_js_error!(serde_wasm_bindgen::from_value(raw_asset_lock_proof))?;

    let asset_lock_proof_json =
        serde_json::to_value(parsed_asset_lock_proof).map_err(|e| from_dpp_err(e.into()))?;

    let tx_validator = Arc::new(AssetLockTransactionValidator::new(
        state_repository_wrapper.clone(),
    ));

    let validator =
        ChainAssetLockProofStructureValidator::new(state_repository_wrapper, tx_validator)
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
