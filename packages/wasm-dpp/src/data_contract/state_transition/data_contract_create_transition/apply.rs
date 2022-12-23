use std::ops::Deref;

use dpp::data_contract::state_transition::apply_data_contract_create_transition_factory::ApplyDataContractCreateTransition;
use wasm_bindgen::prelude::*;

use crate::{
    state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper},
    DataContractCreateTransitionWasm,
};

#[wasm_bindgen(js_name=ApplyDataContractCreateTransition)]
pub struct ApplyDataContractCreateTransitionWasm(
    ApplyDataContractCreateTransition<ExternalStateRepositoryLikeWrapper>,
);

impl From<ApplyDataContractCreateTransition<ExternalStateRepositoryLikeWrapper>>
    for ApplyDataContractCreateTransitionWasm
{
    fn from(wa: ApplyDataContractCreateTransition<ExternalStateRepositoryLikeWrapper>) -> Self {
        ApplyDataContractCreateTransitionWasm(wa)
    }
}

#[wasm_bindgen(js_class=ApplyDataContractCreateTransition)]
impl ApplyDataContractCreateTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        state_repository: ExternalStateRepositoryLike,
    ) -> ApplyDataContractCreateTransitionWasm {
        let pinned_js_state_repository = ExternalStateRepositoryLikeWrapper::new(state_repository);
        ApplyDataContractCreateTransition::new(pinned_js_state_repository).into()
    }

    #[wasm_bindgen(js_name=applyDataContractCreateTransition)]
    pub async fn apply_data_contract_create_transition(
        &self,
        transition: DataContractCreateTransitionWasm,
    ) -> Result<(), JsError> {
        self.0
            .apply_data_contract_create_transition(&transition.into())
            .await
            .map_err(|e| e.deref().into())
    }
}
