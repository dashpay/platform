use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_try_from_js_value;
use crate::impl_wasm_type_info;
use crate::shielded::orchard_action::{SerializedOrchardActionWasm, actions_from_js_options};
use crate::state_transitions::batch::token_base_transition::TokenBaseTransitionWasm;
use crate::utils::{try_from_options, try_vec_to_fixed_bytes};
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_shielded_transfer_transition::v0::v0_methods::TokenShieldedTransferTransitionV0Methods;
use dpp::state_transition::batch_transition::token_shielded_transfer_transition::TokenShieldedTransferTransitionV0;
use dpp::state_transition::batch_transition::TokenShieldedTransferTransition;
use serde::Deserialize;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TOKEN_SHIELDED_TRANSFER_OPTIONS_TS: &str = r#"
/**
 * Options for constructing a TokenShieldedTransferTransition. Spends and re-creates notes inside the token's shielded pool; the identity only pays the fee.
 */
export interface TokenShieldedTransferTransitionOptions {
    base: TokenBaseTransition;
    actions: SerializedOrchardAction[];
    anchor: Uint8Array;
    proof: Uint8Array;
    bindingSignature: Uint8Array;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenShieldedTransferTransitionOptions")]
    pub type TokenShieldedTransferTransitionOptionsJs;
}

/// Non-WASM-instance fields extracted from the constructor options via serde.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenShieldedTransferTransitionSimpleFields {
    anchor: Vec<u8>,
    proof: Vec<u8>,
    binding_signature: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name = "TokenShieldedTransferTransition")]
pub struct TokenShieldedTransferTransitionWasm(TokenShieldedTransferTransition);

impl From<TokenShieldedTransferTransition> for TokenShieldedTransferTransitionWasm {
    fn from(transition: TokenShieldedTransferTransition) -> Self {
        Self(transition)
    }
}

impl From<TokenShieldedTransferTransitionWasm> for TokenShieldedTransferTransition {
    fn from(transition: TokenShieldedTransferTransitionWasm) -> Self {
        transition.0
    }
}

#[wasm_bindgen(js_class = TokenShieldedTransferTransition)]
impl TokenShieldedTransferTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: TokenShieldedTransferTransitionOptionsJs,
    ) -> WasmDppResult<TokenShieldedTransferTransitionWasm> {
        let js_opts: &JsValue = options.as_ref();

        let base: TokenBaseTransitionWasm = try_from_options(js_opts, "base")?;
        let actions = actions_from_js_options(js_opts, "actions")?;

        let fields: TokenShieldedTransferTransitionSimpleFields =
            serde_wasm_bindgen::from_value(options.into())
                .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        let anchor: [u8; 32] = try_vec_to_fixed_bytes(fields.anchor, "anchor")?;
        let binding_signature: [u8; 64] =
            try_vec_to_fixed_bytes(fields.binding_signature, "bindingSignature")?;

        Ok(TokenShieldedTransferTransitionWasm(
            TokenShieldedTransferTransition::V0(TokenShieldedTransferTransitionV0 {
                base: base.into(),
                actions: actions.into_iter().map(Into::into).collect(),
                anchor,
                proof: fields.proof,
                binding_signature,
            }),
        ))
    }

    #[wasm_bindgen(getter = "base")]
    pub fn base(&self) -> TokenBaseTransitionWasm {
        self.0.base().clone().into()
    }

    #[wasm_bindgen(setter = "base")]
    pub fn set_base(&mut self, base: TokenBaseTransitionWasm) {
        self.0.set_base(base.into())
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

impl_try_from_js_value!(
    TokenShieldedTransferTransitionWasm,
    "TokenShieldedTransferTransition"
);
impl_wasm_type_info!(
    TokenShieldedTransferTransitionWasm,
    TokenShieldedTransferTransition
);
