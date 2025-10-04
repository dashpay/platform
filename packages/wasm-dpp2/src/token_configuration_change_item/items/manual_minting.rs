use crate::token_configuration::authorized_action_takers::AuthorizedActionTakersWASM;
use crate::token_configuration_change_item::TokenConfigurationChangeItemWASM;
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
impl TokenConfigurationChangeItemWASM {
    #[wasm_bindgen(js_name = "ManualMintingItem")]
    pub fn manual_minting_item(action_taker: &AuthorizedActionTakersWASM) -> Self {
        TokenConfigurationChangeItemWASM(TokenConfigurationChangeItem::ManualMinting(
            action_taker.clone().into(),
        ))
    }

    #[wasm_bindgen(js_name = "ManualMintingAdminGroupItem")]
    pub fn manual_minting_admin_group_item(action_taker: &AuthorizedActionTakersWASM) -> Self {
        TokenConfigurationChangeItemWASM(TokenConfigurationChangeItem::ManualMintingAdminGroup(
            action_taker.clone().into(),
        ))
    }
}
