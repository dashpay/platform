use std::convert::TryFrom;

use dpp::{document::document_transition::Action, prelude::DataTrigger};
use wasm_bindgen::prelude::*;

use crate::{
    identifier::{identifier_from_js_value, IdentifierWrapper},
    utils::{Inner, WithJsError},
};
pub mod data_trigger_execution_context;
pub mod data_trigger_execution_result;
pub mod execute_data_triggers_factory;
pub mod get_data_triggers_factory;

#[wasm_bindgen(js_name=DataTrigger)]
#[derive(Clone)]
pub struct DataTriggerWasm(DataTrigger);

impl From<DataTrigger> for DataTriggerWasm {
    fn from(value: DataTrigger) -> Self {
        DataTriggerWasm(value)
    }
}

impl From<&DataTrigger> for DataTriggerWasm {
    fn from(value: &DataTrigger) -> Self {
        DataTriggerWasm(value.clone())
    }
}

impl From<DataTriggerWasm> for DataTrigger {
    fn from(value: DataTriggerWasm) -> Self {
        value.0
    }
}

impl From<&DataTriggerWasm> for DataTrigger {
    fn from(value: &DataTriggerWasm) -> Self {
        value.0.clone()
    }
}

#[wasm_bindgen(js_class=DataTrigger)]
impl DataTriggerWasm {
    #[wasm_bindgen(getter=dataContractId)]
    pub fn get_data_contract_id(&self) -> IdentifierWrapper {
        self.0.data_contract_id.into()
    }

    #[wasm_bindgen(setter=dataContractId)]
    pub fn set_data_contract_id(&mut self, id: &JsValue) -> Result<(), JsValue> {
        let id = identifier_from_js_value(id)?;
        self.0.data_contract_id = id;
        Ok(())
    }

    #[wasm_bindgen(getter=documentType)]
    pub fn get_document_type(&self) -> String {
        self.0.document_type.to_string()
    }

    #[wasm_bindgen(setter=documentType)]
    pub fn set_document_type(&mut self, id: String) {
        self.0.document_type = id;
    }

    #[wasm_bindgen(getter=dataTriggerKind)]
    pub fn get_data_trigger_kind(&self) -> String {
        let data_trigger_string: &str = self.0.data_trigger_kind.into();
        data_trigger_string.to_owned()
    }

    #[wasm_bindgen(getter=transitionAction)]
    pub fn get_transition_action(&self) -> String {
        self.0.transition_action.to_string()
    }

    #[wasm_bindgen(setter=transitionAction)]
    pub fn set_transition_action(&mut self, action_string: String) -> Result<(), JsValue> {
        let action = Action::try_from(action_string).with_js_error()?;
        self.0.transition_action = action;
        Ok(())
    }

    #[wasm_bindgen(getter=topLevelIdentity)]
    pub fn get_top_level_identity(&self) -> JsValue {
        if let Some(ref id) = self.0.top_level_identity {
            let id: IdentifierWrapper = id.to_owned().into();
            id.into()
        } else {
            JsValue::undefined()
        }
    }

    #[wasm_bindgen(setter=topLevelIdentity)]
    pub fn set_top_level_identity(&mut self, maybe_id: &JsValue) -> Result<(), JsValue> {
        if maybe_id.is_undefined() || maybe_id.is_null() {
            self.0.top_level_identity = None
        } else {
            let id = identifier_from_js_value(maybe_id)?;
            self.0.top_level_identity = Some(id);
        }
        Ok(())
    }
}

impl Inner for DataTriggerWasm {
    type InnerItem = DataTrigger;

    fn into_inner(self) -> Self::InnerItem {
        self.0
    }

    fn inner(&self) -> &Self::InnerItem {
        &self.0
    }

    fn inner_mut(&mut self) -> &mut Self::InnerItem {
        &mut self.0
    }
}
