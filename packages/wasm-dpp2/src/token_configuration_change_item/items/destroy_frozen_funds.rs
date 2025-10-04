use crate::token_configuration::authorized_action_takers::AuthorizedActionTakersWasm;
use crate::token_configuration_change_item::TokenConfigurationChangeItemWasm;
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_class = TokenConfigurationChangeItem)]
impl TokenConfigurationChangeItemWasm {
    #[wasm_bindgen(js_name = "DestroyFrozenFundsItem")]
    pub fn destroy_frozen_funds_item(action_taker: &AuthorizedActionTakersWasm) -> Self {
        TokenConfigurationChangeItemWasm(TokenConfigurationChangeItem::DestroyFrozenFunds(
            action_taker.clone().into(),
        ))
    }

    #[wasm_bindgen(js_name = "DestroyFrozenFundsAdminGroupItem")]
    pub fn destroy_frozen_funds_admin_group_item(
        action_taker: &AuthorizedActionTakersWasm,
    ) -> Self {
        TokenConfigurationChangeItemWasm(
            TokenConfigurationChangeItem::DestroyFrozenFundsAdminGroup(action_taker.clone().into()),
        )
    }
}
