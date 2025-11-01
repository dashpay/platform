use crate::state_transitions::batch::token_base_transition::TokenBaseTransitionWasm;
use crate::error::WasmDppResult;
use crate::identifier::IdentifierWasm;
use crate::tokens::configuration::TokenConfigurationWasm;
use dpp::prelude::Identifier;
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_mint_transition::v0::v0_methods::TokenMintTransitionV0Methods;
use dpp::state_transition::batch_transition::token_mint_transition::TokenMintTransitionV0;
use dpp::state_transition::batch_transition::TokenMintTransition;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name=TokenMintTransition)]
pub struct TokenMintTransitionWasm(TokenMintTransition);

impl From<TokenMintTransition> for TokenMintTransitionWasm {
    fn from(transition: TokenMintTransition) -> Self {
        Self(transition)
    }
}

impl From<TokenMintTransitionWasm> for TokenMintTransition {
    fn from(transition: TokenMintTransitionWasm) -> Self {
        transition.0
    }
}

#[wasm_bindgen(js_class = TokenMintTransition)]
impl TokenMintTransitionWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "TokenMintTransition".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "TokenMintTransition".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        base: &TokenBaseTransitionWasm,
        js_issued_to_identity_id: &JsValue,
        amount: u64,
        public_note: Option<String>,
    ) -> WasmDppResult<TokenMintTransitionWasm> {
        let issued_to_identity_id: Option<Identifier> =
            match js_issued_to_identity_id.is_undefined() {
                false => Some(IdentifierWasm::try_from(js_issued_to_identity_id)?.into()),
                true => None,
            };

        Ok(TokenMintTransitionWasm(TokenMintTransition::V0(
            TokenMintTransitionV0 {
                base: base.clone().into(),
                issued_to_identity_id,
                amount,
                public_note,
            },
        )))
    }

    #[wasm_bindgen(getter = issuedToIdentityId)]
    pub fn issued_to_identity_id(&self) -> Option<IdentifierWasm> {
        self.0.issued_to_identity_id().map(|id| id.into())
    }

    #[wasm_bindgen(getter = amount)]
    pub fn get_amount(&self) -> u64 {
        self.0.amount()
    }

    #[wasm_bindgen(getter = base)]
    pub fn get_base(&self) -> TokenBaseTransitionWasm {
        self.0.base().clone().into()
    }

    #[wasm_bindgen(getter = publicNote)]
    pub fn get_public_note(&self) -> Option<String> {
        self.clone().0.public_note_owned()
    }

    #[wasm_bindgen(js_name = getRecipientId)]
    pub fn recipient_id(&self, config: &TokenConfigurationWasm) -> WasmDppResult<IdentifierWasm> {
        Ok(self.0.recipient_id(&config.clone().into())?.into())
    }

    #[wasm_bindgen(setter = issuedToIdentityId)]
    pub fn set_issued_to_identity_id(&mut self, js_id: &JsValue) -> WasmDppResult<()> {
        match js_id.is_undefined() {
            true => {
                self.0.set_issued_to_identity_id(None);
            }
            false => {
                let id = IdentifierWasm::try_from(js_id)?;

                self.0.set_issued_to_identity_id(Some(id.into()));
            }
        }

        Ok(())
    }

    #[wasm_bindgen(setter = amount)]
    pub fn set_amount(&mut self, amount: u64) {
        self.0.set_amount(amount)
    }

    #[wasm_bindgen(setter = base)]
    pub fn set_base(&mut self, base: TokenBaseTransitionWasm) {
        self.0.set_base(base.into())
    }

    #[wasm_bindgen(setter = publicNote)]
    pub fn set_public_note(&mut self, note: Option<String>) {
        self.0.set_public_note(note)
    }
}
