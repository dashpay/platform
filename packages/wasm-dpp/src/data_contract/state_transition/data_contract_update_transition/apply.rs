use std::ops::Deref;

use dpp::data_contract::state_transition::apply_data_contract_update_transition_factory::ApplyDataContractUpdateTransition;
use wasm_bindgen::prelude::*;

use crate::{
    state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper},
    DataContractUpdateTransitionWasm,
};

#[wasm_bindgen(js_name=ApplyDataContractUpdateTransition)]
pub struct ApplyDataContractUpdateTransitionWasm(
    ApplyDataContractUpdateTransition<ExternalStateRepositoryLikeWrapper>,
);

impl From<ApplyDataContractUpdateTransition<ExternalStateRepositoryLikeWrapper>>
    for ApplyDataContractUpdateTransitionWasm
{
    fn from(wa: ApplyDataContractUpdateTransition<ExternalStateRepositoryLikeWrapper>) -> Self {
        ApplyDataContractUpdateTransitionWasm(wa)
    }
}

#[wasm_bindgen(js_class=ApplyDataContractUpdateTransition)]
impl ApplyDataContractUpdateTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        state_repository: ExternalStateRepositoryLike,
    ) -> ApplyDataContractUpdateTransitionWasm {
        let pinned_js_state_repository = ExternalStateRepositoryLikeWrapper::new(state_repository);
        ApplyDataContractUpdateTransition::new(pinned_js_state_repository).into()
    }

    #[wasm_bindgen(js_name=applyDataContractUpdateTransition)]
    pub async fn apply_data_contract_update_transition(
        &self,
        transition: DataContractUpdateTransitionWasm,
    ) -> Result<(), JsError> {
        self.0
            .apply_data_contract_update_transition(&transition.into())
            .await
            .map_err(|e| e.deref().into())
    }
}
