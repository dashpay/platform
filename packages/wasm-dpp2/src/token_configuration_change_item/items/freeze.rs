use crate::token_configuration::authorized_action_takers::AuthorizedActionTakersWASM;
use crate::token_configuration_change_item::TokenConfigurationChangeItemWASM;
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_class = TokenConfigurationChangeItem)]
impl TokenConfigurationChangeItemWASM {
    #[wasm_bindgen(js_name = "FreezeItem")]
    pub fn freeze_item(
        action_taker: &AuthorizedActionTakersWASM,
    ) -> TokenConfigurationChangeItemWASM {
        TokenConfigurationChangeItemWASM(TokenConfigurationChangeItem::Freeze(
            action_taker.clone().into(),
        ))
    }

    #[wasm_bindgen(js_name = "FreezeAdminGroupItem")]
    pub fn freeze_admin_group_item(
        action_taker: &AuthorizedActionTakersWASM,
    ) -> TokenConfigurationChangeItemWASM {
        TokenConfigurationChangeItemWASM(TokenConfigurationChangeItem::FreezeAdminGroup(
            action_taker.clone().into(),
        ))
    }
}
