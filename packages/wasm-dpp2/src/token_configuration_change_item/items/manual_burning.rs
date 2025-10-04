use crate::token_configuration::authorized_action_takers::AuthorizedActionTakersWASM;
use crate::token_configuration_change_item::TokenConfigurationChangeItemWASM;
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
impl TokenConfigurationChangeItemWASM {
    #[wasm_bindgen(js_name = "ManualBurningItem")]
    pub fn manual_burning_item(action_taker: &AuthorizedActionTakersWASM) -> Self {
        TokenConfigurationChangeItemWASM(TokenConfigurationChangeItem::ManualBurning(
            action_taker.clone().into(),
        ))
    }

    #[wasm_bindgen(js_name = "ManualBurningAdminGroupItem")]
    pub fn manual_burning_admin_group_item(action_taker: &AuthorizedActionTakersWASM) -> Self {
        TokenConfigurationChangeItemWASM(TokenConfigurationChangeItem::ManualBurningAdminGroup(
            action_taker.clone().into(),
        ))
    }
}
