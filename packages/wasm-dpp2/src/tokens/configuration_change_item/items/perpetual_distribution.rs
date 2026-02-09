use crate::tokens::configuration::authorized_action_takers::AuthorizedActionTakersWasm;
use crate::tokens::configuration::perpetual_distribution::TokenPerpetualDistributionWasm;
use crate::tokens::configuration_change_item::TokenConfigurationChangeItemWasm;
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_class = TokenConfigurationChangeItem)]
impl TokenConfigurationChangeItemWasm {
    #[wasm_bindgen(js_name = "PerpetualDistributionConfigurationItem")]
    pub fn perpetual_distribution_item(
        perpetual_distribution: Option<TokenPerpetualDistributionWasm>,
    ) -> Self {
        TokenConfigurationChangeItemWasm(TokenConfigurationChangeItem::PerpetualDistribution(
            perpetual_distribution.map(|p| p.into()),
        ))
    }

    #[wasm_bindgen(js_name = "PerpetualDistributionControlGroupItem")]
    pub fn perpetual_distribution_control_group_item(
        action_taker: &AuthorizedActionTakersWasm,
    ) -> Self {
        TokenConfigurationChangeItemWasm(
            TokenConfigurationChangeItem::PerpetualDistributionControlGroup(
                action_taker.clone().into(),
            ),
        )
    }

    #[wasm_bindgen(js_name = "PerpetualDistributionAdminGroupItem")]
    pub fn perpetual_distribution_admin_group_item(
        action_taker: &AuthorizedActionTakersWasm,
    ) -> Self {
        TokenConfigurationChangeItemWasm(
            TokenConfigurationChangeItem::PerpetualDistributionAdminGroup(
                action_taker.clone().into(),
            ),
        )
    }
}
