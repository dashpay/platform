use crate::tokens::configuration::authorized_action_takers::AuthorizedActionTakersWasm;
use crate::tokens::configuration_change_item::TokenConfigurationChangeItemWasm;
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_class = TokenConfigurationChangeItem)]
impl TokenConfigurationChangeItemWasm {
    #[wasm_bindgen(js_name = "ManualBurningItem")]
    pub fn manual_burning_item(action_taker: &AuthorizedActionTakersWasm) -> Self {
        TokenConfigurationChangeItemWasm(TokenConfigurationChangeItem::ManualBurning(
            action_taker.clone().into(),
        ))
    }

    #[wasm_bindgen(js_name = "ManualBurningAdminGroupItem")]
    pub fn manual_burning_admin_group_item(action_taker: &AuthorizedActionTakersWasm) -> Self {
        TokenConfigurationChangeItemWasm(TokenConfigurationChangeItem::ManualBurningAdminGroup(
            action_taker.clone().into(),
        ))
    }
}
