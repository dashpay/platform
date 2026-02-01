use crate::error::WasmDppResult;
use crate::identifier::{IdentifierLikeOrUndefinedJs, IdentifierWasm};
use crate::tokens::configuration::authorized_action_takers::AuthorizedActionTakersWasm;
use crate::tokens::configuration_change_item::TokenConfigurationChangeItemWasm;
use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use dpp::prelude::Identifier;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_class = TokenConfigurationChangeItem)]
impl TokenConfigurationChangeItemWasm {
    #[wasm_bindgen(js_name = "NewTokensDestinationIdentityItem")]
    pub fn new_tokens_destination_identity_item(
        identity_id: IdentifierLikeOrUndefinedJs,
    ) -> WasmDppResult<TokenConfigurationChangeItemWasm> {
        let identity_id: Option<Identifier> =
            Option::<IdentifierWasm>::try_from(identity_id)?.map(Into::into);

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
