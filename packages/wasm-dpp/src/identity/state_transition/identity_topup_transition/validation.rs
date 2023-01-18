use dpp::identity::state_transition::asset_lock_proof::{
    AssetLockProofValidator, AssetLockTransactionValidator, ChainAssetLockProofStructureValidator,
    InstantAssetLockProofStructureValidator,
};
use dpp::identity::state_transition::identity_topup_transition::validation::basic::IdentityTopUpTransitionBasicValidator;
use dpp::identity::state_transition::identity_topup_transition::validation::state::validate_identity_topup_transition_state;

use dpp::state_transition::state_transition_execution_context::StateTransitionExecutionContext;

use dpp::version::{ProtocolVersionValidator, COMPATIBILITY_MAP, LATEST_VERSION};
use dpp::ProtocolError;

use std::sync::Arc;
use wasm_bindgen::prelude::*;

use crate::errors::from_dpp_err;
use crate::utils::ToSerdeJSONExt;
use crate::{
    state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper},
    validation::ValidationResultWasm,
    IdentityTopUpTransitionWasm, StateTransitionExecutionContextWasm,
};

#[wasm_bindgen(js_name=validateIdentityTopUpTransitionState)]
pub async fn validate_identity_topup_transition_state_wasm(
    state_repository: ExternalStateRepositoryLike,
    state_transition: &IdentityTopUpTransitionWasm,
) -> Result<ValidationResultWasm, JsValue> {
    let wrapped_state_repository = ExternalStateRepositoryLikeWrapper::new(state_repository);
    let validation_result = validate_identity_topup_transition_state(
        state_transition.to_owned().clone().into(),
        &wrapped_state_repository,
    )
    .await
    .map_err(|e| from_dpp_err(e.into()))?;

    Ok(validation_result.map(|_| JsValue::undefined()).into())
}

#[wasm_bindgen(js_name=validateIdentityTopUpTransitionBasic)]
pub async fn validate_identity_topup_transition_basic_wasm(
    state_repository: ExternalStateRepositoryLike,
    raw_state_transition: JsValue,
    execution_context: &StateTransitionExecutionContextWasm,
) -> Result<ValidationResultWasm, JsValue> {
    let execution_context: &StateTransitionExecutionContext = execution_context.into();

    let state_repository_wrapper =
        Arc::new(ExternalStateRepositoryLikeWrapper::new(state_repository));

    let current_protocol_version =
        js_sys::Reflect::get(&raw_state_transition, &"protocolVersion".into())
            .map(|v| v.as_f64().unwrap_or(LATEST_VERSION as f64) as u32)
            .unwrap_or(LATEST_VERSION);

    let state_transition_json = raw_state_transition.with_serde_to_json_value()?;

    let protocol_version_validator = ProtocolVersionValidator::new(
        current_protocol_version,
        LATEST_VERSION,
        COMPATIBILITY_MAP.clone(),
    );

    let asset_lock_proof_validator: AssetLockProofValidator<ExternalStateRepositoryLikeWrapper>;
    {
        let tx_validator = Arc::new(AssetLockTransactionValidator::new(
            state_repository_wrapper.clone(),
        ));

        let instant_asset_lock_proof_validator = InstantAssetLockProofStructureValidator::new(
            state_repository_wrapper.clone(),
            tx_validator.clone(),
        )
        .map_err(|e| from_dpp_err(ProtocolError::Generic(e.to_string())))?;

        let chain_asset_lock_proof_validator = ChainAssetLockProofStructureValidator::new(
            state_repository_wrapper.clone(),
            tx_validator.clone(),
        )
        .map_err(|e| from_dpp_err(ProtocolError::Generic(e.to_string())))?;

        asset_lock_proof_validator = AssetLockProofValidator::new(
            instant_asset_lock_proof_validator,
            chain_asset_lock_proof_validator,
        );
    }

    let validator = IdentityTopUpTransitionBasicValidator::new(
        Arc::new(protocol_version_validator),
        Arc::new(asset_lock_proof_validator),
    )
    .map_err(|e| from_dpp_err(ProtocolError::Generic(e.to_string())))?;

    let validation_result = validator
        .validate(&state_transition_json, execution_context)
        .await
        .map_err(|e| from_dpp_err(e.into()))?;

    Ok(validation_result.map(|_| JsValue::undefined()).into())
}
