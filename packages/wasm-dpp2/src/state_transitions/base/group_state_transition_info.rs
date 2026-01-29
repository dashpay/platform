use crate::error::WasmDppResult;
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_try_from_js_value;
use crate::impl_wasm_type_info;
use dpp::group::GroupStateTransitionInfo;
use dpp::prelude::Identifier;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name = "GroupStateTransitionInfo")]
pub struct GroupStateTransitionInfoWasm(GroupStateTransitionInfo);

impl From<GroupStateTransitionInfoWasm> for GroupStateTransitionInfo {
    fn from(info: GroupStateTransitionInfoWasm) -> Self {
        info.0
    }
}

impl From<GroupStateTransitionInfo> for GroupStateTransitionInfoWasm {
    fn from(info: GroupStateTransitionInfo) -> Self {
        GroupStateTransitionInfoWasm(info)
    }
}

#[wasm_bindgen(js_class = GroupStateTransitionInfo)]
impl GroupStateTransitionInfoWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        #[wasm_bindgen(js_name = "groupContractPosition")] group_contract_position: u16,
        #[wasm_bindgen(js_name = "actionId")] action_id: IdentifierLikeJs,
        #[wasm_bindgen(js_name = "isActionProposer")] action_is_proposer: bool,
    ) -> WasmDppResult<GroupStateTransitionInfoWasm> {
        let action_id: Identifier = action_id.try_into()?;

        Ok(GroupStateTransitionInfoWasm(GroupStateTransitionInfo {
            group_contract_position,
            action_id,
            action_is_proposer,
        }))
    }

    #[wasm_bindgen(setter = "groupContractPosition")]
    pub fn set_group_contract_position(
        &mut self,
        #[wasm_bindgen(js_name = "groupContractPosition")] group_contract_position: u16,
    ) {
        self.0.group_contract_position = group_contract_position;
    }

    #[wasm_bindgen(setter = "actionId")]
    pub fn set_action_id(
        &mut self,
        #[wasm_bindgen(js_name = "actionId")] action_id: IdentifierLikeJs,
    ) -> WasmDppResult<()> {
        self.0.action_id = action_id.try_into()?;
        Ok(())
    }

    #[wasm_bindgen(setter = "isActionProposer")]
    pub fn set_is_action_proposer(
        &mut self,
        #[wasm_bindgen(js_name = "isActionProposer")] is_action_proposer: bool,
    ) {
        self.0.action_is_proposer = is_action_proposer;
    }

    #[wasm_bindgen(getter = "groupContractPosition")]
    pub fn group_contract_position(&self) -> u16 {
        self.0.group_contract_position
    }

    #[wasm_bindgen(getter = "actionId")]
    pub fn action_id(&self) -> IdentifierWasm {
        self.0.action_id.into()
    }

    #[wasm_bindgen(getter = "isActionProposer")]
    pub fn is_action_proposer(&self) -> bool {
        self.0.action_is_proposer
    }
}

impl_try_from_js_value!(GroupStateTransitionInfoWasm, "GroupStateTransitionInfo");
impl_wasm_type_info!(GroupStateTransitionInfoWasm, GroupStateTransitionInfo);
