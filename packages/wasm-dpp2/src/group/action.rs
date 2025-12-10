use crate::group::action_event::GroupActionEventWasm;
use crate::identifier::IdentifierWasm;
use dpp::data_contract::TokenContractPosition;
use dpp::group::group_action::{GroupAction, GroupActionAccessors};
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, Debug, PartialEq)]
#[wasm_bindgen(js_name = "GroupAction")]
pub struct GroupActionWasm(GroupAction);

impl From<GroupAction> for GroupActionWasm {
    fn from(action: GroupAction) -> Self {
        GroupActionWasm(action)
    }
}

impl From<GroupActionWasm> for GroupAction {
    fn from(action: GroupActionWasm) -> Self {
        action.0
    }
}

#[wasm_bindgen(js_class = GroupAction)]
impl GroupActionWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "GroupAction".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "GroupAction".to_string()
    }

    #[wasm_bindgen(getter = "contractId")]
    pub fn contract_id(&self) -> IdentifierWasm {
        self.0.contract_id().into()
    }

    #[wasm_bindgen(getter = "proposerId")]
    pub fn proposer_id(&self) -> IdentifierWasm {
        self.0.proposer_id().into()
    }

    #[wasm_bindgen(getter = "tokenContractPosition")]
    pub fn token_contract_position(&self) -> TokenContractPosition {
        self.0.token_contract_position()
    }

    #[wasm_bindgen(getter = "event")]
    pub fn event(&self) -> GroupActionEventWasm {
        GroupActionEventWasm::from(self.0.event().clone())
    }
}
