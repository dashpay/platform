use dpp::data_contract::state_transition::apply_data_contract_create_transition_factory::ApplyDataContractCreateTransition;
use wasm_bindgen::prelude::*;

use crate::state_repository::ExternalStateRepositoryLikeWrapper;

#[wasm_bindgen(js_name=ApplyDataContractCreateTransition)]
pub struct ApplyDataContractCreateTransitionWasm(
    ApplyDataContractCreateTransition<ExternalStateRepositoryLikeWrapper>,
);
