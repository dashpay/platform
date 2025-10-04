use crate::token_configuration::authorized_action_takers::AuthorizedActionTakersWASM;
use crate::token_configuration::trade_mode::TokenTradeModeWASM;
use crate::token_configuration_change_item::TokenConfigurationChangeItemWASM;
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_class = TokenConfigurationChangeItem)]
impl TokenConfigurationChangeItemWASM {
    #[wasm_bindgen(js_name = "MarketplaceTradeModeItem")]
    pub fn market_trade_mode_item(trade_mode: &TokenTradeModeWASM) -> Self {
        TokenConfigurationChangeItemWASM(TokenConfigurationChangeItem::MarketplaceTradeMode(
            trade_mode.clone().into(),
        ))
    }

    #[wasm_bindgen(js_name = "MarketplaceTradeModeControlGroupItem")]
    pub fn market_trade_mode_control_group_item(action_taker: &AuthorizedActionTakersWASM) -> Self {
        TokenConfigurationChangeItemWASM(
            TokenConfigurationChangeItem::MarketplaceTradeModeControlGroup(
                action_taker.clone().into(),
            ),
        )
    }

    #[wasm_bindgen(js_name = "MarketplaceTradeModeAdminGroupItem")]
    pub fn market_trade_mode_admin_group_item(action_taker: &AuthorizedActionTakersWASM) -> Self {
        TokenConfigurationChangeItemWASM(
            TokenConfigurationChangeItem::MarketplaceTradeModeAdminGroup(
                action_taker.clone().into(),
            ),
        )
    }
}
