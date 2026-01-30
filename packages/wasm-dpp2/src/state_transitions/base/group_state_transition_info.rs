use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_try_from_js_value;
use crate::impl_wasm_type_info;
use crate::utils::try_from_options;
use dpp::group::GroupStateTransitionInfo;
use serde::Deserialize;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GroupStateTransitionInfoOptionsSerde {
    group_contract_position: u16,
    #[serde(default)]
    is_action_proposer: bool,
}

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * Options for creating a GroupStateTransitionInfo instance.
 */
export interface GroupStateTransitionInfoOptions {
    groupContractPosition: number;
    actionId: IdentifierLike;
    isActionProposer?: boolean;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "GroupStateTransitionInfoOptions")]
    pub type GroupStateTransitionInfoOptionsJs;
}

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
        options: GroupStateTransitionInfoOptionsJs,
    ) -> WasmDppResult<GroupStateTransitionInfoWasm> {
        // Extract complex types first (borrows &options)
        let action_id: IdentifierWasm = try_from_options(&options, "actionId")?;

        // Deserialize primitive fields via serde (consumes options)
        let opts: GroupStateTransitionInfoOptionsSerde =
            serde_wasm_bindgen::from_value(options.into())
                .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        Ok(GroupStateTransitionInfoWasm(GroupStateTransitionInfo {
            group_contract_position: opts.group_contract_position,
            action_id: action_id.into(),
            action_is_proposer: opts.is_action_proposer,
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
