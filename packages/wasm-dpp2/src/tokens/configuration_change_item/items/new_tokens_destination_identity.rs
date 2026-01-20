use crate::error::WasmDppResult;
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::tokens::configuration::authorized_action_takers::AuthorizedActionTakersWasm;
use crate::tokens::configuration_change_item::TokenConfigurationChangeItemWasm;
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use dpp::prelude::Identifier;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_class = TokenConfigurationChangeItem)]
impl TokenConfigurationChangeItemWasm {
    #[wasm_bindgen(js_name = "NewTokensDestinationIdentityItem")]
    pub fn new_tokens_destination_identity_item(
        identity_id: IdentifierLikeJs,
    ) -> WasmDppResult<TokenConfigurationChangeItemWasm> {
        let id_value: JsValue = identity_id.into();
        let identity_id: Option<Identifier> = match id_value.is_undefined() {
            true => None,
            false => Some(IdentifierWasm::try_from(&id_value)?.into()),
        };

        Ok(TokenConfigurationChangeItemWasm(
            TokenConfigurationChangeItem::NewTokensDestinationIdentity(identity_id),
        ))
    }

    #[wasm_bindgen(js_name = "NewTokensDestinationIdentityControlGroupItem")]
    pub fn new_tokens_destination_identity_control_group_item(
        action_taker: &AuthorizedActionTakersWasm,
    ) -> Self {
        TokenConfigurationChangeItemWasm(
            TokenConfigurationChangeItem::NewTokensDestinationIdentityControlGroup(
                action_taker.clone().into(),
            ),
        )
    }

    #[wasm_bindgen(js_name = "NewTokensDestinationIdentityAdminGroupItem")]
    pub fn new_tokens_destination_identity_admin_group_item(
        action_taker: &AuthorizedActionTakersWasm,
    ) -> Self {
        TokenConfigurationChangeItemWasm(
            TokenConfigurationChangeItem::NewTokensDestinationIdentityAdminGroup(
                action_taker.clone().into(),
            ),
        )
    }
}
