use crate::group::action_event::GroupActionEventWasm;
use crate::identifier::IdentifierWasm;
use crate::impl_wasm_conversions;
use crate::impl_wasm_type_info;
use dpp::data_contract::TokenContractPosition;
use dpp::group::group_action::{GroupAction, GroupActionAccessors};
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * GroupAction serialized as a plain object.
 */
export interface GroupActionObject {
    contractId: Uint8Array;
    proposerId: Uint8Array;
    tokenContractPosition: number;
    event: GroupActionEventObject;
}

/**
 * GroupAction serialized as JSON.
 */
export interface GroupActionJSON {
    contractId: string;
    proposerId: string;
    tokenContractPosition: number;
    event: GroupActionEventJSON;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "GroupActionObject")]
    pub type GroupActionObjectJs;

    #[wasm_bindgen(typescript_type = "GroupActionJSON")]
    pub type GroupActionJSONJs;
}

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

impl_wasm_conversions!(
    GroupActionWasm,
    GroupAction,
    GroupActionObjectJs,
    GroupActionJSONJs
);
impl_wasm_type_info!(GroupActionWasm, GroupAction);
