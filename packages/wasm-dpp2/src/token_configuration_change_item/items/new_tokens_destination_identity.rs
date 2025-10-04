use crate::identifier::IdentifierWASM;
use crate::token_configuration::authorized_action_takers::AuthorizedActionTakersWASM;
use crate::token_configuration_change_item::TokenConfigurationChangeItemWASM;
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use dpp::prelude::Identifier;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_class = TokenConfigurationChangeItem)]
impl TokenConfigurationChangeItemWASM {
    #[wasm_bindgen(js_name = "NewTokensDestinationIdentityItem")]
    pub fn new_tokens_destination_identity_item(
        js_identity_id: &JsValue,
    ) -> Result<TokenConfigurationChangeItemWASM, JsValue> {
        let identity_id: Option<Identifier> = match js_identity_id.is_undefined() {
            true => None,
            false => Some(IdentifierWASM::try_from(js_identity_id)?.into()),
        };

        Ok(TokenConfigurationChangeItemWASM(
            TokenConfigurationChangeItem::NewTokensDestinationIdentity(identity_id),
        ))
    }

    #[wasm_bindgen(js_name = "NewTokensDestinationIdentityControlGroupItem")]
    pub fn new_tokens_destination_identity_control_group_item(
        action_taker: &AuthorizedActionTakersWASM,
    ) -> Self {
        TokenConfigurationChangeItemWASM(
            TokenConfigurationChangeItem::NewTokensDestinationIdentityControlGroup(
                action_taker.clone().into(),
            ),
        )
    }

    #[wasm_bindgen(js_name = "NewTokensDestinationIdentityAdminGroupItem")]
    pub fn new_tokens_destination_identity_admin_group_item(
        action_taker: &AuthorizedActionTakersWASM,
    ) -> Self {
        TokenConfigurationChangeItemWASM(
            TokenConfigurationChangeItem::NewTokensDestinationIdentityAdminGroup(
                action_taker.clone().into(),
            ),
        )
    }
}
