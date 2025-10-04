use crate::token_configuration::authorized_action_takers::AuthorizedActionTakersWASM;
use crate::token_configuration::configuration_convention::TokenConfigurationConventionWASM;
use crate::token_configuration_change_item::TokenConfigurationChangeItemWASM;
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
impl TokenConfigurationChangeItemWASM {
    #[wasm_bindgen(js_name = "conventionsItem")]
    pub fn conventions_item(convention: &TokenConfigurationConventionWASM) -> Self {
        TokenConfigurationChangeItemWASM(TokenConfigurationChangeItem::Conventions(
            convention.clone().into(),
        ))
    }

    #[wasm_bindgen(js_name = "ConventionsAdminGroupItem")]
    pub fn conventions_admin_group_item(action_taker: &AuthorizedActionTakersWASM) -> Self {
        TokenConfigurationChangeItemWASM(TokenConfigurationChangeItem::ConventionsAdminGroup(
            action_taker.clone().into(),
        ))
    }

    #[wasm_bindgen(js_name = "ConventionsControlGroupItem")]
    pub fn conventions_control_group_item(action_taker: &AuthorizedActionTakersWASM) -> Self {
        TokenConfigurationChangeItemWASM(TokenConfigurationChangeItem::ConventionsControlGroup(
            action_taker.clone().into(),
        ))
    }
}
