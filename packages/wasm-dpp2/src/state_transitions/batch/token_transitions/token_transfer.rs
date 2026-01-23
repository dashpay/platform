use crate::error::WasmDppResult;
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_wasm_type_info;
use crate::state_transitions::batch::token_base_transition::TokenBaseTransitionWasm;
use crate::tokens::encrypted_note::private_encrypted_note::PrivateEncryptedNoteWasm;
use crate::tokens::encrypted_note::shared_encrypted_note::SharedEncryptedNoteWasm;
use crate::utils::{IntoWasm, get_optional_property, get_required_property, try_to_object, try_to_u64};
use dpp::prelude::Identifier;
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_transfer_transition::v0::v0_methods::TokenTransferTransitionV0Methods;
use dpp::state_transition::batch_transition::token_transfer_transition::TokenTransferTransitionV0;
use dpp::state_transition::batch_transition::TokenTransferTransition;
use dpp::tokens::{PrivateEncryptedNote, SharedEncryptedNote};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen(typescript_custom_section)]
const TOKEN_TRANSFER_OPTIONS_TS: &'static str = r#"
export interface TokenTransferTransitionOptions {
    base: TokenBaseTransition;
    recipientId: IdentifierLike;
    amount: bigint | number;
    publicNote?: string;
    sharedEncryptedNote?: SharedEncryptedNote;
    privateEncryptedNote?: PrivateEncryptedNote;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenTransferTransitionOptions")]
    pub type TokenTransferTransitionOptionsJs;
}

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name = "TokenTransferTransition")]
pub struct TokenTransferTransitionWasm(TokenTransferTransition);

impl From<TokenTransferTransition> for TokenTransferTransitionWasm {
    fn from(transition: TokenTransferTransition) -> Self {
        Self(transition)
    }
}

impl From<TokenTransferTransitionWasm> for TokenTransferTransition {
    fn from(transition: TokenTransferTransitionWasm) -> Self {
        transition.0
    }
}

#[wasm_bindgen(js_class = TokenTransferTransition)]
impl TokenTransferTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: TokenTransferTransitionOptionsJs,
    ) -> WasmDppResult<TokenTransferTransitionWasm> {
        let options_obj = try_to_object(options.into(), "options")?;

        let base_js = get_required_property(&options_obj, "base")?;
        let base = base_js
            .to_wasm::<TokenBaseTransitionWasm>("TokenBaseTransition")?
            .clone();

        let recipient_id_js = get_required_property(&options_obj, "recipientId")?;
        let recipient_id: Identifier = IdentifierWasm::try_from(&recipient_id_js)?.into();

        let amount = try_to_u64(get_required_property(&options_obj, "amount")?)?;

        let public_note_js = get_optional_property(&options_obj, "publicNote");
        let public_note: Option<String> = if public_note_js.is_undefined() {
            None
        } else {
            public_note_js.as_string()
        };

        let js_shared_encrypted_note = get_optional_property(&options_obj, "sharedEncryptedNote");
        let shared_encrypted_note: Option<SharedEncryptedNote> =
            match js_shared_encrypted_note.is_undefined() {
                true => None,
                false => Some(
                    js_shared_encrypted_note
                        .to_wasm::<SharedEncryptedNoteWasm>("SharedEncryptedNote")?
                        .clone()
                        .into(),
                ),
            };

        let js_private_encrypted_note = get_optional_property(&options_obj, "privateEncryptedNote");
        let private_encrypted_note: Option<PrivateEncryptedNote> =
            match js_private_encrypted_note.is_undefined() {
                true => None,
                false => Some(
                    js_private_encrypted_note
                        .to_wasm::<PrivateEncryptedNoteWasm>("PrivateEncryptedNote")?
                        .clone()
                        .into(),
                ),
            };

        Ok(TokenTransferTransitionWasm(TokenTransferTransition::V0(
            TokenTransferTransitionV0 {
                base: base.into(),
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
    pub fn get_base(&self) -> TokenBaseTransitionWasm {
        self.0.base().clone().into()
    }

    #[wasm_bindgen(getter = "publicNote")]
    pub fn get_public_note(&self) -> Option<String> {
        self.clone().0.public_note_owned()
    }

    #[wasm_bindgen(getter = "sharedEncryptedNote")]
    pub fn get_shared_encrypted_note(&self) -> Option<SharedEncryptedNoteWasm> {
        self.clone()
            .0
            .shared_encrypted_note_owned()
            .map(|shared_encrypted_note| shared_encrypted_note.into())
    }

    #[wasm_bindgen(getter = "privateEncryptedNote")]
    pub fn get_private_encrypted_note(&self) -> Option<PrivateEncryptedNoteWasm> {
        self.clone()
            .0
            .private_encrypted_note_owned()
            .map(|private_encrypted_note| private_encrypted_note.into())
    }

    #[wasm_bindgen(getter = recipientId)]
    pub fn recipient_id(&self) -> IdentifierWasm {
        self.0.recipient_id().into()
    }

    #[wasm_bindgen(setter = recipientId)]
    pub fn set_recipient_id(&mut self, recipient_id: IdentifierLikeJs) -> WasmDppResult<()> {
        self.0.set_recipient_id(recipient_id.try_into()?);
        Ok(())
    }

    #[wasm_bindgen(setter = "amount")]
    pub fn set_amount(&mut self, amount: u64) {
        self.0.set_amount(amount)
    }

    #[wasm_bindgen(setter = "base")]
    pub fn set_base(&mut self, base: TokenBaseTransitionWasm) {
        self.0.set_base(base.into())
    }

    #[wasm_bindgen(setter = "publicNote")]
    pub fn set_public_note(&mut self, note: Option<String>) {
        self.0.set_public_note(note)
    }

    #[wasm_bindgen(setter = "sharedEncryptedNote")]
    pub fn set_shared_encrypted_note(
        &mut self,
        shared_encrypted_note: &JsValue,
    ) -> WasmDppResult<()> {
        let shared_encrypted_note: Option<SharedEncryptedNote> =
            match shared_encrypted_note.is_undefined() {
                true => None,
                false => Some(
                    shared_encrypted_note
                        .to_wasm::<SharedEncryptedNoteWasm>("SharedEncryptedNote")?
                        .clone()
                        .into(),
                ),
            };

        self.0.set_shared_encrypted_note(shared_encrypted_note);
        Ok(())
    }

    #[wasm_bindgen(setter = "privateEncryptedNote")]
    pub fn set_private_encrypted_note(
        &mut self,
        private_encrypted_note: &JsValue,
    ) -> WasmDppResult<()> {
        let private_encrypted_note: Option<PrivateEncryptedNote> =
            match private_encrypted_note.is_undefined() {
                true => None,
                false => Some(
                    private_encrypted_note
                        .to_wasm::<PrivateEncryptedNoteWasm>("PrivateEncryptedNote")?
                        .clone()
                        .into(),
                ),
            };

        self.0.set_private_encrypted_note(private_encrypted_note);
        Ok(())
    }
}

impl_wasm_type_info!(TokenTransferTransitionWasm, TokenTransferTransition);
