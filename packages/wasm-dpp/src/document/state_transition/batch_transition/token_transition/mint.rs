use crate::identifier::IdentifierWrapper;
use crate::tokens::TokenConfigurationWasm;
use crate::utils::WithJsError;
use dpp::state_transition::state_transitions::document::batch_transition::token_mint_transition::v0::v0_methods::TokenMintTransitionV0Methods;
use dpp::state_transition::state_transitions::document::batch_transition::TokenMintTransition;
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

#[wasm_bindgen]
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
}
