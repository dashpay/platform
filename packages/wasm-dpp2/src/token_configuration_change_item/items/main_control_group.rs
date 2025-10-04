use crate::token_configuration_change_item::TokenConfigurationChangeItemWasm;
use dpp::data_contract::GroupContractPosition;
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_class = TokenConfigurationChangeItem)]
impl TokenConfigurationChangeItemWasm {
    #[wasm_bindgen(js_name = "MainControlGroupItem")]
    pub fn main_control_group_item(group_contract_position: Option<GroupContractPosition>) -> Self {
        TokenConfigurationChangeItemWasm(TokenConfigurationChangeItem::MainControlGroup(
            group_contract_position,
        ))
    }
}
