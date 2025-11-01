use crate::tokens::configuration::authorized_action_takers::AuthorizedActionTakersWasm;
use crate::tokens::configuration_change_item::TokenConfigurationChangeItemWasm;
use dpp::balances::credits::TokenAmount;
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_class = TokenConfigurationChangeItem)]
impl TokenConfigurationChangeItemWasm {
    #[wasm_bindgen(js_name = "MaxSupplyItem")]
    pub fn max_supply_item(supply: Option<TokenAmount>) -> Self {
        TokenConfigurationChangeItemWasm(TokenConfigurationChangeItem::MaxSupply(supply))
    }

    #[wasm_bindgen(js_name = "MaxSupplyControlGroupItem")]
    pub fn max_supply_control_group_item(action_taker: &AuthorizedActionTakersWasm) -> Self {
        TokenConfigurationChangeItemWasm(TokenConfigurationChangeItem::MaxSupplyControlGroup(
            action_taker.clone().into(),
        ))
    }

    #[wasm_bindgen(js_name = "MaxSupplyAdminGroupItem")]
    pub fn max_supply_admin_group_item(action_taker: &AuthorizedActionTakersWasm) -> Self {
        TokenConfigurationChangeItemWasm(TokenConfigurationChangeItem::MaxSupplyAdminGroup(
            action_taker.clone().into(),
        ))
    }
}
