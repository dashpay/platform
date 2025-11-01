use crate::tokens::configuration::authorized_action_takers::AuthorizedActionTakersWasm;
use crate::tokens::configuration_change_item::TokenConfigurationChangeItemWasm;
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_class = TokenConfigurationChangeItem)]
impl TokenConfigurationChangeItemWasm {
    #[wasm_bindgen(js_name = "MintingAllowChoosingDestinationItem")]
    pub fn minting_allow_choosing_destination_item(flag: bool) -> Self {
        TokenConfigurationChangeItemWasm(
            TokenConfigurationChangeItem::MintingAllowChoosingDestination(flag),
        )
    }

    #[wasm_bindgen(js_name = "MintingAllowChoosingDestinationControlGroupItem")]
    pub fn minting_allow_choosing_destination_control_group_item(
        action_taker: &AuthorizedActionTakersWasm,
    ) -> Self {
        TokenConfigurationChangeItemWasm(
            TokenConfigurationChangeItem::MintingAllowChoosingDestinationControlGroup(
                action_taker.clone().into(),
            ),
        )
    }

    #[wasm_bindgen(js_name = "MintingAllowChoosingDestinationAdminGroupItem")]
    pub fn minting_allow_choosing_destination_admin_group_item(
        action_taker: &AuthorizedActionTakersWasm,
    ) -> Self {
        TokenConfigurationChangeItemWasm(
            TokenConfigurationChangeItem::MintingAllowChoosingDestinationAdminGroup(
                action_taker.clone().into(),
            ),
        )
    }
}
