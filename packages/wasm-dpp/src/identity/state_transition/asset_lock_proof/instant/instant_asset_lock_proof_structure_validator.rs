use dpp::identity::state_transition::asset_lock_proof::{
    AssetLockTransactionValidator, InstantAssetLockProofStructureValidator,
};
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

#[wasm_bindgen(js_name = InstantAssetLockProofStructureValidator)]
pub struct InstantAssetLockProofStructureValidatorWasm(
    InstantAssetLockProofStructureValidator<ExternalStateRepositoryLikeWrapper>,
);

impl From<InstantAssetLockProofStructureValidator<ExternalStateRepositoryLikeWrapper>>
    for InstantAssetLockProofStructureValidatorWasm
{
    fn from(
        validator: InstantAssetLockProofStructureValidator<ExternalStateRepositoryLikeWrapper>,
    ) -> Self {
        Self(validator)
    }
}

#[wasm_bindgen(js_class = InstantAssetLockProofStructureValidator)]
impl InstantAssetLockProofStructureValidatorWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        state_repository: ExternalStateRepositoryLike,
    ) -> Result<InstantAssetLockProofStructureValidatorWasm, JsValue> {
        let state_repository_wrapper =
            Arc::new(ExternalStateRepositoryLikeWrapper::new(state_repository));

        let tx_validator = Arc::new(AssetLockTransactionValidator::new(
            state_repository_wrapper.clone(),
        ));

        let validator =
            InstantAssetLockProofStructureValidator::new(state_repository_wrapper, tx_validator)
                .map_err(|e| from_dpp_err(ProtocolError::Generic(e.to_string())))?;

        Ok(validator.into())
    }

    #[wasm_bindgen]
    pub async fn validate(
        &self,
        raw_asset_lock_proof: JsValue,
        execution_context: &StateTransitionExecutionContextWasm,
    ) -> Result<ValidationResultWasm, JsValue> {
        let asset_lock_proof_object = raw_asset_lock_proof.with_serde_to_platform_value()?;

        let context: &StateTransitionExecutionContext = execution_context.into();
        let validation_result = self
            .0
            .validate(&asset_lock_proof_object, context)
            .await
            .map_err(|e| from_dpp_err(e.into()))?;

        Ok(validation_result
            .map(|item| JsValue::from(Buffer::from_bytes(&item)))
            .into())
    }
}
