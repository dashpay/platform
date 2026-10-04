use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::shielded::orchard_action::{SerializedOrchardActionWasm, actions_from_js_options};
use crate::state_transitions::StateTransitionWasm;
use crate::utils::{try_from_options, try_vec_to_fixed_bytes};
use crate::{impl_wasm_conversions_inner, impl_wasm_type_info};
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::serialization::{PlatformDeserializableUntrusted, PlatformSerializable};
use dpp::state_transition::token_purchase_from_shielded_pool_transition::TokenPurchaseFromShieldedPoolTransition;
use dpp::state_transition::token_purchase_from_shielded_pool_transition::accessors::TokenPurchaseFromShieldedPoolTransitionAccessorsV0;
use dpp::state_transition::token_purchase_from_shielded_pool_transition::v0::TokenPurchaseFromShieldedPoolTransitionV0;
use dpp::state_transition::{StateTransition, StateTransitionLike};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * Options for constructing a TokenPurchaseFromShieldedPoolTransition (token purchase paid from the credit pool). Two Orchard
 * bundles: the token pool bundle and the credit pool fee bundle. There is no platform signature.
 */
export interface TokenPurchaseFromShieldedPoolTransitionOptions {
    dataContractId: IdentifierLike;
    tokenContractPosition: number;
    tokenId: IdentifierLike;
    tokenCount: bigint;
    totalAgreedPrice: bigint;
    tokenActions: SerializedOrchardAction[];
    tokenAnchor: Uint8Array;
    tokenProof: Uint8Array;
    tokenBindingSignature: Uint8Array;
    feeActions: SerializedOrchardAction[];
    feeAnchor: Uint8Array;
    feeProof: Uint8Array;
    feeBindingSignature: Uint8Array;
    creditAmount: bigint;
}

export interface TokenPurchaseFromShieldedPoolTransitionObject {
    $formatVersion: string;
    dataContractId: Uint8Array;
    tokenContractPosition: number;
    tokenId: Uint8Array;
    tokenCount: bigint;
    totalAgreedPrice: bigint;
    tokenActions: SerializedOrchardActionObject[];
    tokenAnchor: Uint8Array;
    tokenProof: Uint8Array;
    tokenBindingSignature: Uint8Array;
    feeActions: SerializedOrchardActionObject[];
    feeAnchor: Uint8Array;
    feeProof: Uint8Array;
    feeBindingSignature: Uint8Array;
    creditAmount: bigint;
}

export interface TokenPurchaseFromShieldedPoolTransitionJSON {
    $formatVersion: string;
    dataContractId: string;
    tokenContractPosition: number;
    tokenId: string;
    tokenCount: number | string;
    totalAgreedPrice: number | string;
    tokenActions: SerializedOrchardActionJSON[];
    tokenAnchor: string;
    tokenProof: string;
    tokenBindingSignature: string;
    feeActions: SerializedOrchardActionJSON[];
    feeAnchor: string;
    feeProof: string;
    feeBindingSignature: string;
    creditAmount: number | string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "TokenPurchaseFromShieldedPoolTransitionOptions")]
    pub type TokenPurchaseFromShieldedPoolTransitionOptionsJs;

    #[wasm_bindgen(typescript_type = "TokenPurchaseFromShieldedPoolTransitionObject")]
    pub type TokenPurchaseFromShieldedPoolTransitionObjectJs;

    #[wasm_bindgen(typescript_type = "TokenPurchaseFromShieldedPoolTransitionJSON")]
    pub type TokenPurchaseFromShieldedPoolTransitionJSONJs;
}

/// Non-WASM-instance fields extracted from the constructor options via serde.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenPurchaseFromShieldedPoolTransitionSimpleFields {
    token_contract_position: u16,
    token_count: u64,
    total_agreed_price: u64,
    token_anchor: Vec<u8>,
    token_proof: Vec<u8>,
    token_binding_signature: Vec<u8>,
    fee_anchor: Vec<u8>,
    fee_proof: Vec<u8>,
    fee_binding_signature: Vec<u8>,
    credit_amount: u64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(transparent)]
#[wasm_bindgen(js_name = TokenPurchaseFromShieldedPoolTransition)]
pub struct TokenPurchaseFromShieldedPoolTransitionWasm(TokenPurchaseFromShieldedPoolTransition);

impl From<TokenPurchaseFromShieldedPoolTransition> for TokenPurchaseFromShieldedPoolTransitionWasm {
    fn from(v: TokenPurchaseFromShieldedPoolTransition) -> Self {
        TokenPurchaseFromShieldedPoolTransitionWasm(v)
    }
}

impl From<TokenPurchaseFromShieldedPoolTransitionWasm> for TokenPurchaseFromShieldedPoolTransition {
    fn from(v: TokenPurchaseFromShieldedPoolTransitionWasm) -> Self {
        v.0
    }
}

#[wasm_bindgen(js_class = TokenPurchaseFromShieldedPoolTransition)]
impl TokenPurchaseFromShieldedPoolTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        options: TokenPurchaseFromShieldedPoolTransitionOptionsJs,
    ) -> WasmDppResult<TokenPurchaseFromShieldedPoolTransitionWasm> {
        let js_opts: &JsValue = options.as_ref();
        let data_contract_id: IdentifierWasm = try_from_options(js_opts, "dataContractId")?;
        let token_id: IdentifierWasm = try_from_options(js_opts, "tokenId")?;
        let token_actions = actions_from_js_options(js_opts, "tokenActions")?;
        let fee_actions = actions_from_js_options(js_opts, "feeActions")?;
        let fields: TokenPurchaseFromShieldedPoolTransitionSimpleFields =
            serde_wasm_bindgen::from_value(options.into())
                .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;
        let token_anchor: [u8; 32] = try_vec_to_fixed_bytes(fields.token_anchor, "tokenAnchor")?;
        let token_binding_signature: [u8; 64] =
            try_vec_to_fixed_bytes(fields.token_binding_signature, "tokenBindingSignature")?;
        let fee_anchor: [u8; 32] = try_vec_to_fixed_bytes(fields.fee_anchor, "feeAnchor")?;
        let fee_binding_signature: [u8; 64] =
            try_vec_to_fixed_bytes(fields.fee_binding_signature, "feeBindingSignature")?;
        Ok(TokenPurchaseFromShieldedPoolTransitionWasm(
            TokenPurchaseFromShieldedPoolTransition::V0(
                TokenPurchaseFromShieldedPoolTransitionV0 {
                    data_contract_id: data_contract_id.into(),
                    token_contract_position: fields.token_contract_position,
                    token_id: token_id.into(),
                    token_count: fields.token_count,
                    total_agreed_price: fields.total_agreed_price,
                    token_actions: token_actions.into_iter().map(Into::into).collect(),
                    token_anchor,
                    token_proof: fields.token_proof,
                    token_binding_signature,
                    fee_actions: fee_actions.into_iter().map(Into::into).collect(),
                    fee_anchor,
                    fee_proof: fields.fee_proof,
                    fee_binding_signature,
                    credit_amount: fields.credit_amount,
                },
            ),
        ))
    }

    #[wasm_bindgen(getter = "dataContractId")]
    pub fn data_contract_id(&self) -> IdentifierWasm {
        self.0.data_contract_id().into()
    }

    #[wasm_bindgen(getter = "tokenContractPosition")]
    pub fn token_contract_position(&self) -> u16 {
        self.0.token_contract_position()
    }

    #[wasm_bindgen(getter = "tokenId")]
    pub fn token_id(&self) -> IdentifierWasm {
        self.0.token_id().into()
    }

    #[wasm_bindgen(getter = "tokenCount")]
    pub fn token_count(&self) -> u64 {
        self.0.token_count()
    }

    #[wasm_bindgen(getter = "totalAgreedPrice")]
    pub fn total_agreed_price(&self) -> u64 {
        self.0.total_agreed_price()
    }

    /// The serialized Orchard actions of the token pool bundle.
    #[wasm_bindgen(getter = "tokenActions")]
    pub fn token_actions(&self) -> Vec<SerializedOrchardActionWasm> {
        self.0
            .token_actions()
            .iter()
            .cloned()
            .map(SerializedOrchardActionWasm::from)
            .collect()
    }

    #[wasm_bindgen(getter = "tokenAnchor")]
    pub fn token_anchor(&self) -> Vec<u8> {
        self.0.token_anchor().to_vec()
    }

    #[wasm_bindgen(getter = "tokenProof")]
    pub fn token_proof(&self) -> Vec<u8> {
        self.0.token_proof().to_vec()
    }

    #[wasm_bindgen(getter = "tokenBindingSignature")]
    pub fn token_binding_signature(&self) -> Vec<u8> {
        self.0.token_binding_signature().to_vec()
    }

    /// The serialized Orchard actions of the credit pool fee bundle.
    #[wasm_bindgen(getter = "feeActions")]
    pub fn fee_actions(&self) -> Vec<SerializedOrchardActionWasm> {
        self.0
            .fee_actions()
            .iter()
            .cloned()
            .map(SerializedOrchardActionWasm::from)
            .collect()
    }

    #[wasm_bindgen(getter = "feeAnchor")]
    pub fn fee_anchor(&self) -> Vec<u8> {
        self.0.fee_anchor().to_vec()
    }

    #[wasm_bindgen(getter = "feeProof")]
    pub fn fee_proof(&self) -> Vec<u8> {
        self.0.fee_proof().to_vec()
    }

    #[wasm_bindgen(getter = "feeBindingSignature")]
    pub fn fee_binding_signature(&self) -> Vec<u8> {
        self.0.fee_binding_signature().to_vec()
    }

    /// Credits leaving the credit pool: the fee bundle's value balance.
    #[wasm_bindgen(getter = "creditAmount")]
    pub fn credit_amount(&self) -> u64 {
        self.0.credit_amount()
    }

    #[wasm_bindgen(js_name = getModifiedDataIds)]
    pub fn modified_data_ids(&self) -> Vec<IdentifierWasm> {
        self.0
            .modified_data_ids()
            .into_iter()
            .map(IdentifierWasm::from)
            .collect()
    }

    #[wasm_bindgen(js_name = toBytes)]
    pub fn to_bytes(&self) -> WasmDppResult<Vec<u8>> {
        Ok(PlatformSerializable::serialize_to_bytes(
            &StateTransition::TokenPurchaseFromShieldedPool(self.0.clone()),
        )?)
    }

    #[wasm_bindgen(js_name = "toHex")]
    pub fn to_hex(&self) -> WasmDppResult<String> {
        Ok(encode(self.to_bytes()?.as_slice(), Hex))
    }

    #[wasm_bindgen(js_name = "toBase64")]
    pub fn to_base64(&self) -> WasmDppResult<String> {
        Ok(encode(self.to_bytes()?.as_slice(), Base64))
    }

    #[wasm_bindgen(js_name = fromBytes)]
    pub fn from_bytes(
        bytes: Vec<u8>,
    ) -> WasmDppResult<TokenPurchaseFromShieldedPoolTransitionWasm> {
        let st = StateTransition::deserialize_from_bytes_untrusted(&bytes)?;
        match st {
            StateTransition::TokenPurchaseFromShieldedPool(inner) => Ok(inner.into()),
            _ => Err(WasmDppError::invalid_argument(
                "Invalid state transition type: expected TokenPurchaseFromShieldedPool",
            )),
        }
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> WasmDppResult<TokenPurchaseFromShieldedPoolTransitionWasm> {
        let bytes =
            decode(hex.as_str(), Hex).map_err(|e| WasmDppError::serialization(e.to_string()))?;
        TokenPurchaseFromShieldedPoolTransitionWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(
        base64: String,
    ) -> WasmDppResult<TokenPurchaseFromShieldedPoolTransitionWasm> {
        let bytes = decode(base64.as_str(), Base64)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        TokenPurchaseFromShieldedPoolTransitionWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = toStateTransition)]
    pub fn to_state_transition(&self) -> StateTransitionWasm {
        StateTransition::TokenPurchaseFromShieldedPool(self.0.clone()).into()
    }

    #[wasm_bindgen(js_name = "fromStateTransition")]
    pub fn from_state_transition(
        st: &StateTransitionWasm,
    ) -> WasmDppResult<TokenPurchaseFromShieldedPoolTransitionWasm> {
        let rs_st: StateTransition = st.clone().into();
        match rs_st {
            StateTransition::TokenPurchaseFromShieldedPool(inner) => Ok(inner.into()),
            _ => Err(WasmDppError::invalid_argument(
                "Invalid state transition type: expected TokenPurchaseFromShieldedPool",
            )),
        }
    }
}

impl_wasm_conversions_inner!(
    TokenPurchaseFromShieldedPoolTransitionWasm,
    TokenPurchaseFromShieldedPoolTransition,
    TokenPurchaseFromShieldedPoolTransition,
    TokenPurchaseFromShieldedPoolTransitionObjectJs,
    TokenPurchaseFromShieldedPoolTransitionJSONJs
);
impl_wasm_type_info!(
    TokenPurchaseFromShieldedPoolTransitionWasm,
    TokenPurchaseFromShieldedPoolTransition
);
