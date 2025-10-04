use crate::identifier::IdentifierWASM;
use dpp::group::GroupStateTransitionInfo;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name=GroupStateTransitionInfo)]
pub struct GroupStateTransitionInfoWASM(GroupStateTransitionInfo);

impl From<GroupStateTransitionInfoWASM> for GroupStateTransitionInfo {
    fn from(info: GroupStateTransitionInfoWASM) -> Self {
        info.0
    }
}

impl From<GroupStateTransitionInfo> for GroupStateTransitionInfoWASM {
    fn from(info: GroupStateTransitionInfo) -> Self {
        GroupStateTransitionInfoWASM(info)
    }
}

#[wasm_bindgen(js_class = GroupStateTransitionInfo)]
impl GroupStateTransitionInfoWASM {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "GroupStateTransitionInfo".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "GroupStateTransitionInfo".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        group_contract_position: u16,
        action_id: &JsValue,
        action_is_proposer: bool,
    ) -> Result<GroupStateTransitionInfoWASM, JsValue> {
        Ok(GroupStateTransitionInfoWASM(GroupStateTransitionInfo {
            group_contract_position,
            action_id: IdentifierWASM::try_from(action_id)?.into(),
            action_is_proposer,
        }))
    }

    #[wasm_bindgen(setter = "groupContractPosition")]
    pub fn set_group_contract_position(&mut self, group_contract_position: u16) {
        self.0.group_contract_position = group_contract_position;
    }

    #[wasm_bindgen(setter = "actionId")]
    pub fn set_action_id(&mut self, action_id: &JsValue) -> Result<(), JsValue> {
        self.0.action_id = IdentifierWASM::try_from(action_id)?.into();
        Ok(())
    }

    #[wasm_bindgen(setter = "actionIsProposer")]
    pub fn set_action_is_proposer(&mut self, action_is_proposer: bool) {
        self.0.action_is_proposer = action_is_proposer;
    }

    #[wasm_bindgen(getter = "groupContractPosition")]
    pub fn get_group_contract_position(&mut self) -> u16 {
        self.0.group_contract_position
    }

    #[wasm_bindgen(getter = "actionId")]
    pub fn get_action_id(&self) -> IdentifierWASM {
        self.0.action_id.into()
    }

    #[wasm_bindgen(getter = "actionIsProposer")]
    pub fn get_action_is_proposer(&self) -> bool {
        self.0.action_is_proposer
    }
}
