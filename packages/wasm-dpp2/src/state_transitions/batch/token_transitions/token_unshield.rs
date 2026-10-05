use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_try_from_js_value;
use crate::impl_wasm_type_info;
use crate::shielded::orchard_action::{SerializedOrchardActionWasm, actions_from_js_options};
use crate::state_transitions::batch::token_base_transition::TokenBaseTransitionWasm;
use crate::utils::{try_from_options, try_to_u64, try_vec_to_fixed_bytes};
use dpp::prelude::Identifier;
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_unshield_transition::v0::v0_methods::TokenUnshieldTransitionV0Methods;
use dpp::state_transition::batch_transition::token_unshield_transition::TokenUnshieldTransitionV0;
use dpp::state_transition::batch_transition::TokenUnshieldTransition;
use serde::Deserialize;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TOKEN_UNSHIELD_OPTIONS_TS: &str = r#"
/**
 * Options for constructing a TokenUnshieldTransition. Spends shielded notes and credits `amount` tokens to `recipientId`.
 */
export interface TokenUnshieldTransitionOptions {
    base: TokenBaseTransition;
    amount: bigint;
    recipientId: IdentifierLike;
    actions: SerializedOrchardAction[];
    anchor: Uint8Array;
    proof: Uint8Array;
    bindingSignature: Uint8Array;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenUnshieldTransitionOptions")]
    pub type TokenUnshieldTransitionOptionsJs;
}

/// Non-WASM-instance fields extracted from the constructor options via serde.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenUnshieldTransitionSimpleFields {
    amount: u64,
    anchor: Vec<u8>,
    proof: Vec<u8>,
    binding_signature: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name = "TokenUnshieldTransition")]
pub struct TokenUnshieldTransitionWasm(TokenUnshieldTransition);

impl From<TokenUnshieldTransition> for TokenUnshieldTransitionWasm {
    fn from(transition: TokenUnshieldTransition) -> Self {
        Self(transition)
    }
}

impl From<TokenUnshieldTransitionWasm> for TokenUnshieldTransition {
    fn from(transition: TokenUnshieldTransitionWasm) -> Self {
        transition.0
    }
}

#[wasm_bindgen(js_class = TokenUnshieldTransition)]
impl TokenUnshieldTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: TokenUnshieldTransitionOptionsJs,
    ) -> WasmDppResult<TokenUnshieldTransitionWasm> {
        let js_opts: &JsValue = options.as_ref();

        let base: TokenBaseTransitionWasm = try_from_options(js_opts, "base")?;
        let recipient_id: IdentifierWasm = try_from_options(js_opts, "recipientId")?;
        let actions = actions_from_js_options(js_opts, "actions")?;

        let fields: TokenUnshieldTransitionSimpleFields =
            serde_wasm_bindgen::from_value(options.into())
                .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        let anchor: [u8; 32] = try_vec_to_fixed_bytes(fields.anchor, "anchor")?;
        let binding_signature: [u8; 64] =
            try_vec_to_fixed_bytes(fields.binding_signature, "bindingSignature")?;

        Ok(TokenUnshieldTransitionWasm(TokenUnshieldTransition::V0(
            TokenUnshieldTransitionV0 {
                base: base.into(),
                amount: fields.amount,
                recipient_id: recipient_id.into(),
                actions: actions.into_iter().map(Into::into).collect(),
                anchor,
                proof: fields.proof,
                binding_signature,
            },
        )))
    }

    #[wasm_bindgen(getter = "base")]
    pub fn base(&self) -> TokenBaseTransitionWasm {
        self.0.base().clone().into()
    }

    #[wasm_bindgen(setter = "base")]
    pub fn set_base(&mut self, base: TokenBaseTransitionWasm) {
        self.0.set_base(base.into())
    }

    /// Tokens crossing the pool boundary.
    #[wasm_bindgen(getter = "amount")]
    pub fn amount(&self) -> u64 {
        self.0.amount()
    }

    #[wasm_bindgen(setter = "amount")]
    pub fn set_amount(&mut self, amount: &js_sys::BigInt) -> WasmDppResult<()> {
        self.0.set_amount(try_to_u64(amount, "amount")?);
        Ok(())
    }

    /// Identity credited with the unshielded tokens.
    #[wasm_bindgen(getter = "recipientId")]
    pub fn recipient_id(&self) -> IdentifierWasm {
        self.0.recipient_id().into()
    }

    #[wasm_bindgen(setter = "recipientId")]
    pub fn set_recipient_id(&mut self, recipient_id: IdentifierLikeJs) -> WasmDppResult<()> {
        let recipient_id: Identifier = recipient_id.try_into()?;
        self.0.set_recipient_id(recipient_id);
        Ok(())
    }

    /// The serialized Orchard actions.
    #[wasm_bindgen(getter = "actions")]
    pub fn actions(&self) -> Vec<SerializedOrchardActionWasm> {
        self.0
            .actions()
            .iter()
            .cloned()
            .map(SerializedOrchardActionWasm::from)
            .collect()
    }

    /// The Orchard anchor (32-byte note commitment tree root) the bundle was built against.
    #[wasm_bindgen(getter = "anchor")]
    pub fn anchor(&self) -> Vec<u8> {
        self.0.anchor().to_vec()
    }

    /// The Halo 2 proof bytes.
    #[wasm_bindgen(getter = "proof")]
    pub fn proof(&self) -> Vec<u8> {
        self.0.proof().to_vec()
    }

    /// The RedPallas binding signature (64 bytes).
    #[wasm_bindgen(getter = "bindingSignature")]
    pub fn binding_signature(&self) -> Vec<u8> {
        self.0.binding_signature().to_vec()
    }
}

impl_try_from_js_value!(TokenUnshieldTransitionWasm, "TokenUnshieldTransition");
impl_wasm_type_info!(TokenUnshieldTransitionWasm, TokenUnshieldTransition);
