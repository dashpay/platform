use crate::tokens::configuration::authorized_action_takers::AuthorizedActionTakersWasm;
use crate::tokens::configuration::trade_mode::TokenTradeModeWasm;
use crate::tokens::configuration_change_item::TokenConfigurationChangeItemWasm;
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_class = TokenConfigurationChangeItem)]
impl TokenConfigurationChangeItemWasm {
    #[wasm_bindgen(js_name = "MarketplaceTradeModeItem")]
    pub fn market_trade_mode_item(trade_mode: &TokenTradeModeWasm) -> Self {
        TokenConfigurationChangeItemWasm(TokenConfigurationChangeItem::MarketplaceTradeMode(
            trade_mode.clone().into(),
        ))
    }

    #[wasm_bindgen(js_name = "MarketplaceTradeModeControlGroupItem")]
    pub fn market_trade_mode_control_group_item(action_taker: &AuthorizedActionTakersWasm) -> Self {
        TokenConfigurationChangeItemWasm(
            TokenConfigurationChangeItem::MarketplaceTradeModeControlGroup(
                action_taker.clone().into(),
            ),
        )
    }

    #[wasm_bindgen(js_name = "MarketplaceTradeModeAdminGroupItem")]
    pub fn market_trade_mode_admin_group_item(action_taker: &AuthorizedActionTakersWasm) -> Self {
        TokenConfigurationChangeItemWasm(
            TokenConfigurationChangeItem::MarketplaceTradeModeAdminGroup(
                action_taker.clone().into(),
            ),
        )
    }
}
