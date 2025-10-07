use crate::tokens::configuration::authorized_action_takers::AuthorizedActionTakersWasm;
use crate::tokens::configuration::configuration_convention::TokenConfigurationConventionWasm;
use crate::tokens::configuration_change_item::TokenConfigurationChangeItemWasm;
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_class = TokenConfigurationChangeItem)]
impl TokenConfigurationChangeItemWasm {
    #[wasm_bindgen(js_name = "conventionsItem")]
    pub fn conventions_item(convention: &TokenConfigurationConventionWasm) -> Self {
        TokenConfigurationChangeItemWasm(TokenConfigurationChangeItem::Conventions(
            convention.clone().into(),
        ))
    }

    #[wasm_bindgen(js_name = "ConventionsAdminGroupItem")]
    pub fn conventions_admin_group_item(action_taker: &AuthorizedActionTakersWasm) -> Self {
        TokenConfigurationChangeItemWasm(TokenConfigurationChangeItem::ConventionsAdminGroup(
            action_taker.clone().into(),
        ))
    }

    #[wasm_bindgen(js_name = "ConventionsControlGroupItem")]
    pub fn conventions_control_group_item(action_taker: &AuthorizedActionTakersWasm) -> Self {
        TokenConfigurationChangeItemWasm(TokenConfigurationChangeItem::ConventionsControlGroup(
            action_taker.clone().into(),
        ))
    }
}
