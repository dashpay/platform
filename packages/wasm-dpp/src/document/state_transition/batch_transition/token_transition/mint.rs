use crate::identifier::IdentifierWrapper;
use crate::tokens::TokenConfigurationWasm;
use crate::utils::WithJsError;
use dpp::state_transition::batch_transition::token_mint_transition::v0::v0_methods::TokenMintTransitionV0Methods;
use dpp::state_transition::batch_transition::TokenMintTransition;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name=TokenMintTransition)]
#[derive(Debug, Clone)]
pub struct TokenMintTransitionWasm(TokenMintTransition);

impl From<TokenMintTransition> for TokenMintTransitionWasm {
    fn from(value: TokenMintTransition) -> Self {
        Self(value)
    }
}

#[wasm_bindgen(js_class=TokenMintTransition)]
impl TokenMintTransitionWasm {
    #[wasm_bindgen(js_name=getRecipientId)]
    pub fn recipient_id(
        &self,
        token_configuration: TokenConfigurationWasm,
    ) -> Result<IdentifierWrapper, JsValue> {
        self.0
            .recipient_id(&token_configuration.into())
            .with_js_error()
            .map(Into::into)
    }

    #[wasm_bindgen(js_name=getIssuedToIdentityId)]
    pub fn issued_to_identity_id(
        &self,
    ) -> Option<IdentifierWrapper> {
        match self.0.issued_to_identity_id() {
            Some(id) => Some(id.into()),
            None => None
        }
    }

    #[wasm_bindgen(js_name=getPublicNote)]
    pub fn public_note(&self) -> Option<String> {
        match self.0.public_note() {
            Some(note) => Some(note.clone()),
            None => None,
        }
    }

    #[wasm_bindgen(js_name=getAmount)]
    pub fn amount(&self) -> u64 {
        self.0.amount()
    }
}
