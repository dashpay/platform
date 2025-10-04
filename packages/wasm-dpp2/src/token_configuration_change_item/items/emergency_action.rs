use crate::token_configuration::authorized_action_takers::AuthorizedActionTakersWASM;
use crate::token_configuration_change_item::TokenConfigurationChangeItemWASM;
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_class = TokenConfigurationChangeItem)]
impl TokenConfigurationChangeItemWASM {
    #[wasm_bindgen(js_name = "EmergencyActionItem")]
    pub fn emergency_action_item(
        action_taker: &AuthorizedActionTakersWASM,
    ) -> TokenConfigurationChangeItemWASM {
        TokenConfigurationChangeItemWASM(TokenConfigurationChangeItem::EmergencyAction(
            action_taker.clone().into(),
        ))
    }

    #[wasm_bindgen(js_name = "EmergencyActionAdminGroupItem")]
    pub fn emergency_action_admin_group_item(
        action_taker: &AuthorizedActionTakersWASM,
    ) -> TokenConfigurationChangeItemWASM {
        TokenConfigurationChangeItemWASM(TokenConfigurationChangeItem::EmergencyActionAdminGroup(
            action_taker.clone().into(),
        ))
    }
}
