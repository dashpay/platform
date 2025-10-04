use crate::batch::token_base_transition::TokenBaseTransitionWASM;
use dpp::prelude::Identifier;
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::TokenTransferTransition;
use dpp::state_transition::batch_transition::token_transfer_transition::TokenTransferTransitionV0;
use dpp::state_transition::batch_transition::token_transfer_transition::v0::v0_methods::TokenTransferTransitionV0Methods;
use dpp::tokens::{PrivateEncryptedNote, SharedEncryptedNote};
use crate::identifier::IdentifierWASM;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;
use crate::encrypted_note::private_encrypted_note::PrivateEncryptedNoteWASM;
use crate::encrypted_note::shared_encrypted_note::SharedEncryptedNoteWASM;
use crate::utils::IntoWasm;

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name=TokenTransferTransition)]
pub struct TokenTransferTransitionWASM(TokenTransferTransition);

impl From<TokenTransferTransition> for TokenTransferTransitionWASM {
    fn from(transition: TokenTransferTransition) -> Self {
        Self(transition)
    }
}

impl From<TokenTransferTransitionWASM> for TokenTransferTransition {
    fn from(transition: TokenTransferTransitionWASM) -> Self {
        transition.0
    }
}

#[wasm_bindgen(js_class = TokenTransferTransition)]
impl TokenTransferTransitionWASM {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "TokenTransferTransition".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "TokenTransferTransition".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        base: &TokenBaseTransitionWASM,
        js_recipient_id: &JsValue,
        amount: u64,
        public_note: Option<String>,
        js_shared_encrypted_note: &JsValue,
        js_private_encrypted_note: &JsValue,
    ) -> Result<TokenTransferTransitionWASM, JsValue> {
        let recipient_id: Identifier = IdentifierWASM::try_from(js_recipient_id)?.into();

        let shared_encrypted_note: Option<SharedEncryptedNote> =
            match js_shared_encrypted_note.is_undefined() {
                true => None,
                false => Some(
                    js_shared_encrypted_note
                        .to_wasm::<SharedEncryptedNoteWASM>("SharedEncryptedNote")?
                        .clone()
                        .into(),
                ),
            };

        let private_encrypted_note: Option<PrivateEncryptedNote> =
            match js_private_encrypted_note.is_undefined() {
                true => None,
                false => Some(
                    js_private_encrypted_note
                        .to_wasm::<PrivateEncryptedNoteWASM>("PrivateEncryptedNote")?
                        .clone()
                        .into(),
                ),
            };

        Ok(TokenTransferTransitionWASM(TokenTransferTransition::V0(
            TokenTransferTransitionV0 {
                base: base.clone().into(),
                recipient_id,
                amount,
                public_note,
                shared_encrypted_note,
                private_encrypted_note,
            },
        )))
    }

    #[wasm_bindgen(getter = "amount")]
    pub fn get_amount(&self) -> u64 {
        self.0.amount()
    }

    #[wasm_bindgen(getter = "base")]
    pub fn get_base(&self) -> TokenBaseTransitionWASM {
        self.0.base().clone().into()
    }

    #[wasm_bindgen(getter = "publicNote")]
    pub fn get_public_note(&self) -> Option<String> {
        self.clone().0.public_note_owned()
    }

    #[wasm_bindgen(getter = "sharedEncryptedNote")]
    pub fn get_shared_encrypted_note(&self) -> Option<SharedEncryptedNoteWASM> {
        match self.clone().0.shared_encrypted_note_owned() {
            Some(shared_encrypted_note) => Some(shared_encrypted_note.into()),
            None => None,
        }
    }

    #[wasm_bindgen(getter = "privateEncryptedNote")]
    pub fn get_private_encrypted_note(&self) -> Option<PrivateEncryptedNoteWASM> {
        match self.clone().0.private_encrypted_note_owned() {
            Some(private_encrypted_note) => Some(private_encrypted_note.into()),
            None => None,
        }
    }

    #[wasm_bindgen(getter = recipientId)]
    pub fn recipient_id(&self) -> IdentifierWASM {
        self.0.recipient_id().into()
    }

    #[wasm_bindgen(setter = recipientId)]
    pub fn set_recipient_id(&mut self, js_recipient: &JsValue) -> Result<(), JsValue> {
        let recipient = IdentifierWASM::try_from(js_recipient)?;

        self.0.set_recipient_id(recipient.into());

        Ok(())
    }

    #[wasm_bindgen(setter = "amount")]
    pub fn set_amount(&mut self, amount: u64) {
        self.0.set_amount(amount)
    }

    #[wasm_bindgen(setter = "base")]
    pub fn set_base(&mut self, base: TokenBaseTransitionWASM) {
        self.0.set_base(base.into())
    }

    #[wasm_bindgen(setter = "publicNote")]
    pub fn set_public_note(&mut self, note: Option<String>) {
        self.0.set_public_note(note)
    }

    #[wasm_bindgen(setter = "sharedEncryptedNote")]
    pub fn set_shared_encrypted_note(
        &mut self,
        js_shared_encrypted_note: &JsValue,
    ) -> Result<(), JsValue> {
        let shared_encrypted_note: Option<SharedEncryptedNote> =
            match js_shared_encrypted_note.is_undefined() {
                true => None,
                false => Some(
                    js_shared_encrypted_note
                        .to_wasm::<SharedEncryptedNoteWASM>("SharedEncryptedNote")?
                        .clone()
                        .into(),
                ),
            };

        Ok(self.0.set_shared_encrypted_note(shared_encrypted_note))
    }

    #[wasm_bindgen(setter = "privateEncryptedNote")]
    pub fn set_private_encrypted_note(
        &mut self,
        js_private_encrypted_note: &JsValue,
    ) -> Result<(), JsValue> {
        let private_encrypted_note: Option<PrivateEncryptedNote> =
            match js_private_encrypted_note.is_undefined() {
                true => None,
                false => Some(
                    js_private_encrypted_note
                        .to_wasm::<PrivateEncryptedNoteWASM>("PrivateEncryptedNote")?
                        .clone()
                        .into(),
                ),
            };

        Ok(self.0.set_private_encrypted_note(private_encrypted_note))
    }
}
