use std::sync::Arc;

use dpp::{
    data_trigger::DataTriggerExecutionContext, schema::data_contract,
    state_repository::StateRepositoryLike,
};
use wasm_bindgen::prelude::*;

use crate::{
    identifier::{identifier_from_js_value, IdentifierWrapper},
    state_repository::{ExternalStateRepositoryLike, ExternalStateRepositoryLikeWrapper},
    utils::Inner,
    DataContractWasm, StateTransitionExecutionContextWasm,
};

#[wasm_bindgen(js_name=DataTriggerExecutionContext)]
pub struct DataTriggerExecutionContextWasm {
    state_repository: Arc<ExternalStateRepositoryLike>,
    owner_id: IdentifierWrapper,
    data_contract: DataContractWasm,
    state_transition_execution_context: StateTransitionExecutionContextWasm,
}

#[wasm_bindgen(js_class=DataTriggerExecutionContext)]
impl DataTriggerExecutionContextWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        state_repository: ExternalStateRepositoryLike,
        js_owner_id: &JsValue,
        data_contract: DataContractWasm,
        state_transition_execution_context: StateTransitionExecutionContextWasm,
    ) -> Result<DataTriggerExecutionContextWasm, JsValue> {
        let owner_id: IdentifierWrapper = identifier_from_js_value(js_owner_id)?.into();

        Ok(Self {
            state_repository: Arc::new(state_repository),
            owner_id,
            data_contract,
            state_transition_execution_context,
        })
    }
    // TODO setters and getters
}

impl DataTriggerExecutionContextWasm {
    pub fn state_repository(&self) -> Arc<ExternalStateRepositoryLike> {
        self.state_repository.clone()
    }
}

pub(crate) fn data_trigger_context_from_wasm<'a, SR>(
    value: &'a DataTriggerExecutionContextWasm,
    external_state_repository: &'a SR,
) -> DataTriggerExecutionContext<'a, SR>
where
    SR: StateRepositoryLike,
{
    DataTriggerExecutionContext {
        data_contract: value.data_contract.inner(),
        owner_id: value.owner_id.inner(),
        state_transition_execution_context: value.state_transition_execution_context.inner(),
        state_repository: external_state_repository,
    }
}
