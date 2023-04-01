use std::sync::Arc;

use dpp::identifier::Identifier;
use dpp::{data_trigger::DataTriggerExecutionContext, state_repository::StateRepositoryLike};
use wasm_bindgen::prelude::*;

use crate::{
    identifier::{identifier_from_js_value, IdentifierWrapper},
    state_repository::ExternalStateRepositoryLike,
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

    #[wasm_bindgen(getter, js_name=ownerId)]
    pub fn get_owner_id(&self) -> IdentifierWrapper {
        self.owner_id.clone()
    }

    #[wasm_bindgen(setter, js_name=ownerId)]
    pub fn set_owner_id(&mut self, owner_id: &JsValue) -> Result<(), JsValue> {
        let id = identifier_from_js_value(owner_id)?;
        self.owner_id = id.into();
        Ok(())
    }

    #[wasm_bindgen(getter, js_name=dataContract)]
    pub fn get_data_contract(&self) -> DataContractWasm {
        self.data_contract.clone()
    }

    #[wasm_bindgen(setter, js_name=dataContract)]
    pub fn set_data_contract(&mut self, data_contract: &DataContractWasm) {
        self.data_contract = data_contract.clone();
    }

    #[wasm_bindgen(getter, js_name=stateTransitionExecutionContext)]
    pub fn get_state_transition_execution_context(&self) -> StateTransitionExecutionContextWasm {
        self.state_transition_execution_context.clone()
    }

    #[wasm_bindgen(setter, js_name=statTransitionExecutionContext)]
    pub fn set_state_transition_execution_context(
        &mut self,
        context: &StateTransitionExecutionContextWasm,
    ) {
        self.state_transition_execution_context = context.clone();
    }
}

impl DataTriggerExecutionContextWasm {
    pub fn state_repository(&self) -> Arc<ExternalStateRepositoryLike> {
        self.state_repository.clone()
    }
}

pub(crate) fn data_trigger_context_from_wasm<'a, SR>(
    value: &'a DataTriggerExecutionContextWasm,
    external_state_repository: &'a SR,
    owner_id: &'a Identifier,
) -> DataTriggerExecutionContext<'a, SR>
where
    SR: StateRepositoryLike,
{
    DataTriggerExecutionContext {
        data_contract: value.data_contract.inner(),
        owner_id,
        state_transition_execution_context: value.state_transition_execution_context.inner(),
        state_repository: external_state_repository,
    }
}
