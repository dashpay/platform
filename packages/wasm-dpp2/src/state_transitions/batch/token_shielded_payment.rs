use crate::error::{WasmDppError, WasmDppResult};
use crate::impl_try_from_js_value;
use crate::impl_wasm_conversions_inner;
use crate::impl_wasm_type_info;
use crate::shielded::orchard_action::{SerializedOrchardActionWasm, actions_from_js_options};
use crate::utils::{try_to_u64, try_vec_to_fixed_bytes};
use dpp::tokens::token_payment_info::v1::TokenShieldedPayment;
use serde::Deserialize;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const TOKEN_SHIELDED_PAYMENT_TS: &str = r#"
/**
 * Options for constructing a TokenShieldedPayment: an Orchard spend bundle in the payment
 * token's shielded pool whose value balance (`amount`) pays a document action's token cost.
 */
export interface TokenShieldedPaymentOptions {
    amount: bigint;
    actions: SerializedOrchardAction[];
    anchor: Uint8Array;
    proof: Uint8Array;
    bindingSignature: Uint8Array;
}

/**
 * TokenShieldedPayment serialized as a plain object.
 */
export interface TokenShieldedPaymentObject {
    amount: bigint;
    actions: SerializedOrchardActionObject[];
    anchor: Uint8Array;
    proof: Uint8Array;
    bindingSignature: Uint8Array;
}

/**
 * TokenShieldedPayment serialized as JSON.
 */
export interface TokenShieldedPaymentJSON {
    amount: number | string;
    actions: SerializedOrchardActionJSON[];
    anchor: string;
    proof: string;
    bindingSignature: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenShieldedPaymentOptions")]
    pub type TokenShieldedPaymentOptionsJs;

    #[wasm_bindgen(typescript_type = "TokenShieldedPaymentObject")]
    pub type TokenShieldedPaymentObjectJs;

    #[wasm_bindgen(typescript_type = "TokenShieldedPaymentJSON")]
    pub type TokenShieldedPaymentJSONJs;
}

/// Non-WASM-instance fields extracted from the constructor options via serde.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenShieldedPaymentSimpleFields {
    amount: u64,
    anchor: Vec<u8>,
    proof: Vec<u8>,
    binding_signature: Vec<u8>,
}

/// A spend bundle in a token's shielded pool paying a document action's token cost.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
#[wasm_bindgen(js_name = "TokenShieldedPayment")]
pub struct TokenShieldedPaymentWasm(TokenShieldedPayment);

impl From<TokenShieldedPayment> for TokenShieldedPaymentWasm {
    fn from(payment: TokenShieldedPayment) -> Self {
        Self(payment)
    }
}

impl From<TokenShieldedPaymentWasm> for TokenShieldedPayment {
    fn from(payment: TokenShieldedPaymentWasm) -> Self {
        payment.0
    }
}

#[wasm_bindgen(js_class = TokenShieldedPayment)]
impl TokenShieldedPaymentWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: TokenShieldedPaymentOptionsJs,
    ) -> WasmDppResult<TokenShieldedPaymentWasm> {
        let js_opts: &JsValue = options.as_ref();
        let actions = actions_from_js_options(js_opts, "actions")?;

        let fields: TokenShieldedPaymentSimpleFields =
            serde_wasm_bindgen::from_value(options.into())
                .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        let anchor: [u8; 32] = try_vec_to_fixed_bytes(fields.anchor, "anchor")?;
        let binding_signature: [u8; 64] =
            try_vec_to_fixed_bytes(fields.binding_signature, "bindingSignature")?;

        Ok(TokenShieldedPaymentWasm(TokenShieldedPayment {
            amount: fields.amount,
            actions: actions.into_iter().map(Into::into).collect(),
            anchor,
            proof: fields.proof,
            binding_signature,
        }))
    }

    /// The tokens the bundle pays (its value balance): the document action's token cost.
    #[wasm_bindgen(getter = "amount")]
    pub fn amount(&self) -> u64 {
        self.0.amount
    }

    #[wasm_bindgen(setter = "amount")]
    pub fn set_amount(&mut self, amount: &js_sys::BigInt) -> WasmDppResult<()> {
        self.0.amount = try_to_u64(amount, "amount")?;
        Ok(())
    }

    /// The serialized Orchard actions.
    #[wasm_bindgen(getter = "actions")]
    pub fn actions(&self) -> Vec<SerializedOrchardActionWasm> {
        self.0
            .actions
            .iter()
            .cloned()
            .map(SerializedOrchardActionWasm::from)
            .collect()
    }

    /// The 32-byte Orchard anchor the bundle was built against.
    #[wasm_bindgen(getter = "anchor")]
    pub fn anchor(&self) -> Vec<u8> {
        self.0.anchor.to_vec()
    }

    /// The Halo 2 proof bytes.
    #[wasm_bindgen(getter = "proof")]
    pub fn proof(&self) -> Vec<u8> {
        self.0.proof.clone()
    }

    /// The 64-byte RedPallas binding signature.
    #[wasm_bindgen(getter = "bindingSignature")]
    pub fn binding_signature(&self) -> Vec<u8> {
        self.0.binding_signature.to_vec()
    }
}

impl_try_from_js_value!(TokenShieldedPaymentWasm, "TokenShieldedPayment");
impl_wasm_type_info!(TokenShieldedPaymentWasm, TokenShieldedPayment);
impl_wasm_conversions_inner!(
    TokenShieldedPaymentWasm,
    TokenShieldedPayment,
    TokenShieldedPayment,
    TokenShieldedPaymentObjectJs,
    TokenShieldedPaymentJSONJs
);
