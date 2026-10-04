use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_try_from_js_value;
use crate::impl_wasm_type_info;
use crate::shielded::orchard_action::{SerializedOrchardActionWasm, actions_from_js_options};
use crate::state_transitions::batch::token_base_transition::TokenBaseTransitionWasm;
use crate::utils::{try_from_options, try_vec_to_fixed_bytes};
use crate::enums::token::distribution_type::TokenDistributionTypeWasm;
use crate::utils::{try_from_options_optional, try_from_options_optional_with, try_to_string};
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_claim_to_pool_transition::v0::v0_methods::TokenClaimToPoolTransitionV0Methods;
use dpp::state_transition::batch_transition::token_claim_to_pool_transition::TokenClaimToPoolTransitionV0;
use dpp::state_transition::batch_transition::TokenClaimToPoolTransition;
use serde::Deserialize;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TOKEN_CLAIM_TO_POOL_OPTIONS_TS: &str = r#"
/**
 * Options for constructing a TokenClaimToPoolTransition. Claims a distribution straight into the token's shielded pool; a perpetual claim names the cycle-aligned moment it claims up to.
 */
export interface TokenClaimToPoolTransitionOptions {
    base: TokenBaseTransition;
    distributionType?: TokenDistributionTypeLike;
    claimUpTo?: bigint;
    publicNote?: string;
    actions: SerializedOrchardAction[];
    anchor: Uint8Array;
    proof: Uint8Array;
    bindingSignature: Uint8Array;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenClaimToPoolTransitionOptions")]
    pub type TokenClaimToPoolTransitionOptionsJs;
}

/// Non-WASM-instance fields extracted from the constructor options via serde.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenClaimToPoolTransitionSimpleFields {
    #[serde(default)]
    claim_up_to: Option<u64>,
    anchor: Vec<u8>,
    proof: Vec<u8>,
    binding_signature: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
#[wasm_bindgen(js_name = "TokenClaimToPoolTransition")]
pub struct TokenClaimToPoolTransitionWasm(TokenClaimToPoolTransition);

impl From<TokenClaimToPoolTransition> for TokenClaimToPoolTransitionWasm {
    fn from(transition: TokenClaimToPoolTransition) -> Self {
        Self(transition)
    }
}

impl From<TokenClaimToPoolTransitionWasm> for TokenClaimToPoolTransition {
    fn from(transition: TokenClaimToPoolTransitionWasm) -> Self {
        transition.0
    }
}

#[wasm_bindgen(js_class = TokenClaimToPoolTransition)]
impl TokenClaimToPoolTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: TokenClaimToPoolTransitionOptionsJs,
    ) -> WasmDppResult<TokenClaimToPoolTransitionWasm> {
        let js_opts: &JsValue = options.as_ref();

        let base: TokenBaseTransitionWasm = try_from_options(js_opts, "base")?;
        let public_note: Option<String> =
            try_from_options_optional_with(js_opts, "publicNote", |v| {
                try_to_string(v, "publicNote")
            })?;
        let distribution_type: TokenDistributionTypeWasm =
            try_from_options_optional(js_opts, "distributionType")?.unwrap_or_default();
        let actions = actions_from_js_options(js_opts, "actions")?;

        let fields: TokenClaimToPoolTransitionSimpleFields =
            serde_wasm_bindgen::from_value(options.into())
                .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        let anchor: [u8; 32] = try_vec_to_fixed_bytes(fields.anchor, "anchor")?;
        let binding_signature: [u8; 64] =
            try_vec_to_fixed_bytes(fields.binding_signature, "bindingSignature")?;

        Ok(TokenClaimToPoolTransitionWasm(
            TokenClaimToPoolTransition::V0(TokenClaimToPoolTransitionV0 {
                base: base.into(),
                distribution_type: distribution_type.into(),
                claim_up_to: fields.claim_up_to,
                public_note,
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

    #[wasm_bindgen(getter = "distributionType")]
    pub fn distribution_type(&self) -> String {
        TokenDistributionTypeWasm::from(self.0.distribution_type()).into()
    }

    /// The moment a perpetual claim pays out to, in the distribution's own unit.
    #[wasm_bindgen(getter = "claimUpTo")]
    pub fn claim_up_to(&self) -> Option<u64> {
        self.0.claim_up_to()
    }

    #[wasm_bindgen(getter = "publicNote")]
    pub fn public_note(&self) -> Option<String> {
        self.0.public_note().cloned()
    }

    #[wasm_bindgen(setter = "publicNote")]
    pub fn set_public_note(&mut self, note: Option<String>) {
        self.0.set_public_note(note)
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

impl_try_from_js_value!(TokenClaimToPoolTransitionWasm, "TokenClaimToPoolTransition");
impl_wasm_type_info!(TokenClaimToPoolTransitionWasm, TokenClaimToPoolTransition);
