use crate::errors::from_dpp_err;
use crate::state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper};
use crate::validation::ValidationResultWasm;
use dpp::identity::state_transition::asset_lock_proof::{
    AssetLockPublicKeyHashFetcher, AssetLockTransactionOutputFetcher,
};
use dpp::state_transition::validation::validate_state_transition_key_signature::StateTransitionKeySignatureValidator;
use dpp::validation::AsyncDataValidator;
use std::sync::Arc;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name=StateTransitionKeySignatureValidator)]
pub struct StateTransitionKeySignatureValidatorWasm(
    StateTransitionKeySignatureValidator<ExternalStateRepositoryLikeWrapper>,
);

impl From<StateTransitionKeySignatureValidator<ExternalStateRepositoryLikeWrapper>>
    for StateTransitionKeySignatureValidatorWasm
{
    fn from(
        validator: StateTransitionKeySignatureValidator<ExternalStateRepositoryLikeWrapper>,
    ) -> Self {
        Self(validator)
    }
}

#[wasm_bindgen(js_class=StateTransitionKeySignatureValidator)]
impl StateTransitionKeySignatureValidatorWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(state_repository: ExternalStateRepositoryLike) -> Self {
        let state_repository_wrapper =
            Arc::new(ExternalStateRepositoryLikeWrapper::new(state_repository));

        let tx_output_fetcher =
            AssetLockTransactionOutputFetcher::new(state_repository_wrapper.clone());

        let public_key_hash_fetcher =
            AssetLockPublicKeyHashFetcher::new(state_repository_wrapper.clone(), tx_output_fetcher);

        StateTransitionKeySignatureValidator::new(state_repository_wrapper, public_key_hash_fetcher)
            .into()
    }

    #[wasm_bindgen]
    pub async fn validate(
        &self,
        state_transition: JsValue,
    ) -> Result<ValidationResultWasm, JsValue> {
        let state_transition =
            super::super::conversion::create_state_transition_from_wasm_instance(
                &state_transition,
            )?;

        let validation_result = self
            .0
            .validate(&state_transition)
            .await
            .map_err(from_dpp_err)?;
        Ok(validation_result.map(|_| JsValue::undefined()).into())
    }
}
