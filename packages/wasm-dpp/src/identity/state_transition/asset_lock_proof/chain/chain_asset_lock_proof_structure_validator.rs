use dpp::identity::state_transition::asset_lock_proof::{
    AssetLockTransactionValidator, ChainAssetLockProofStructureValidator,
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

#[wasm_bindgen(js_name = ChainAssetLockProofStructureValidator)]
pub struct ChainAssetLockProofStructureValidatorWasm(
    ChainAssetLockProofStructureValidator<ExternalStateRepositoryLikeWrapper>,
);

impl From<ChainAssetLockProofStructureValidator<ExternalStateRepositoryLikeWrapper>>
    for ChainAssetLockProofStructureValidatorWasm
{
    fn from(
        validator: ChainAssetLockProofStructureValidator<ExternalStateRepositoryLikeWrapper>,
    ) -> Self {
        Self(validator)
    }
}

#[wasm_bindgen(js_class = ChainAssetLockProofStructureValidator)]
impl ChainAssetLockProofStructureValidatorWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        state_repository: ExternalStateRepositoryLike,
    ) -> Result<ChainAssetLockProofStructureValidatorWasm, JsValue> {
        let state_repository_wrapper =
            Arc::new(ExternalStateRepositoryLikeWrapper::new(state_repository));

        let tx_validator = Arc::new(AssetLockTransactionValidator::new(
            state_repository_wrapper.clone(),
        ));

        let validator =
            ChainAssetLockProofStructureValidator::new(state_repository_wrapper, tx_validator)
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
