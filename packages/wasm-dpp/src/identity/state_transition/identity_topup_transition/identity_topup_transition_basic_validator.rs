use dpp::identity::state_transition::asset_lock_proof::{
    AssetLockProofValidator, AssetLockTransactionValidator, ChainAssetLockProofStructureValidator,
    InstantAssetLockProofStructureValidator,
};
use dpp::identity::state_transition::identity_topup_transition::validation::basic::IdentityTopUpTransitionBasicValidator;

use dpp::version::{ProtocolVersionValidator, COMPATIBILITY_MAP, LATEST_VERSION};
use dpp::ProtocolError;

use std::sync::Arc;
use wasm_bindgen::prelude::*;

use crate::errors::from_dpp_err;
use crate::utils::ToSerdeJSONExt;
use crate::{
    state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper},
    validation::ValidationResultWasm,
    StateTransitionExecutionContextWasm,
};

#[wasm_bindgen(js_name=IdentityTopUpTransitionBasicValidator)]
pub struct IdentityTopUpTransitionBasicValidatorWasm(
    IdentityTopUpTransitionBasicValidator<ExternalStateRepositoryLikeWrapper>,
);

impl From<IdentityTopUpTransitionBasicValidator<ExternalStateRepositoryLikeWrapper>>
    for IdentityTopUpTransitionBasicValidatorWasm
{
    fn from(
        validator: IdentityTopUpTransitionBasicValidator<ExternalStateRepositoryLikeWrapper>,
    ) -> Self {
        Self(validator)
    }
}

#[wasm_bindgen(js_class=IdentityTopUpTransitionBasicValidator)]
impl IdentityTopUpTransitionBasicValidatorWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        state_repository: ExternalStateRepositoryLike,
    ) -> Result<IdentityTopUpTransitionBasicValidatorWasm, JsValue> {
        let state_repository_wrapper =
            Arc::new(ExternalStateRepositoryLikeWrapper::new(state_repository));

        let protocol_version_validator = ProtocolVersionValidator::new(
            LATEST_VERSION,
            LATEST_VERSION,
            COMPATIBILITY_MAP.clone(),
        );

        let asset_lock_proof_validator: AssetLockProofValidator<
            ExternalStateRepositoryLikeWrapper,
        >;
        {
            let tx_validator = Arc::new(AssetLockTransactionValidator::new(
                state_repository_wrapper.clone(),
            ));

            let instant_asset_lock_proof_validator = InstantAssetLockProofStructureValidator::new(
                state_repository_wrapper.clone(),
                tx_validator.clone(),
            )
            .map_err(|e| from_dpp_err(ProtocolError::Generic(e.to_string())))?;

            let chain_asset_lock_proof_validator =
                ChainAssetLockProofStructureValidator::new(state_repository_wrapper, tx_validator)
                    .map_err(|e| from_dpp_err(ProtocolError::Generic(e.to_string())))?;

            asset_lock_proof_validator = AssetLockProofValidator::new(
                instant_asset_lock_proof_validator,
                chain_asset_lock_proof_validator,
            );
        }

        let validator = IdentityTopUpTransitionBasicValidator::new(
            protocol_version_validator,
            Arc::new(asset_lock_proof_validator),
        )
        .map_err(|e| from_dpp_err(ProtocolError::Generic(e.to_string())))?;

        Ok(validator.into())
    }

    #[wasm_bindgen]
    pub async fn validate(
        mut self,
        raw_state_transition: JsValue,
        execution_context: &StateTransitionExecutionContextWasm,
    ) -> Result<ValidationResultWasm, JsValue> {
        let execution_context: &StateTransitionExecutionContext = execution_context.into();

        let current_protocol_version =
            js_sys::Reflect::get(&raw_state_transition, &"protocolVersion".into())
                .map(|v| v.as_f64().unwrap_or(LATEST_VERSION as f64) as u32)
                .unwrap_or(LATEST_VERSION);

        self.0
            .protocol_version_validator()
            .set_current_protocol_version(current_protocol_version);

        let state_transition_object = raw_state_transition.with_serde_to_platform_value()?;

        let validation_result = self
            .0
            .validate(&state_transition_object, execution_context)
            .await
            .map_err(|e| from_dpp_err(e.into()))?;

        Ok(validation_result.map(|_| JsValue::undefined()).into())
    }
}
