use crate::tokens::configuration::authorized_action_takers::AuthorizedActionTakersWasm;
use crate::tokens::configuration_change_item::TokenConfigurationChangeItemWasm;
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_class = TokenConfigurationChangeItem)]
impl TokenConfigurationChangeItemWasm {
    #[wasm_bindgen(js_name = "FreezeItem")]
    pub fn freeze_item(
        action_taker: &AuthorizedActionTakersWasm,
    ) -> TokenConfigurationChangeItemWasm {
        TokenConfigurationChangeItemWasm(TokenConfigurationChangeItem::Freeze(
            action_taker.clone().into(),
        ))
    }

    #[wasm_bindgen(js_name = "FreezeAdminGroupItem")]
    pub fn freeze_admin_group_item(
        action_taker: &AuthorizedActionTakersWasm,
    ) -> TokenConfigurationChangeItemWasm {
        TokenConfigurationChangeItemWasm(TokenConfigurationChangeItem::FreezeAdminGroup(
            action_taker.clone().into(),
        ))
    }
}
