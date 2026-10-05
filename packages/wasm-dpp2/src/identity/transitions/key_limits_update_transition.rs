use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::impl_wasm_conversions_inner;
use crate::impl_wasm_type_info;
use crate::state_transitions::StateTransitionWasm;
use crate::utils::{try_from_options, try_to_u16, try_to_u32, try_to_u64};
use dpp::platform_value::BinaryData;
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::prelude::UserFeeIncrease;
use dpp::serialization::{PlatformDeserializableUntrusted, PlatformSerializable};
use dpp::state_transition::identity_key_limits_update_transition::IdentityKeyLimitsUpdateTransition;
use dpp::state_transition::identity_key_limits_update_transition::accessors::IdentityKeyLimitsUpdateTransitionAccessorsV0;
use dpp::state_transition::identity_key_limits_update_transition::v0::IdentityKeyLimitsUpdateTransitionV0;
use dpp::state_transition::{
    StateTransition, StateTransitionHasUserFeeIncrease, StateTransitionIdentitySigned,
    StateTransitionSingleSigned,
};
use serde::Deserialize;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const KEY_LIMITS_UPDATE_OPTIONS_TS: &str = r#"
/**
 * Raises the limits of one of the identity's authentication keys: the new total budget and the
 * new expiry are absolute values, each greater than the one the key holds, and at least one of
 * them must be given. No identity revision is claimed.
 */
export interface IdentityKeyLimitsUpdateTransitionOptions {
    identityId: IdentifierLike;
    nonce: bigint;
    keyId: number;
    totalBudget?: bigint;
    expiresAt?: bigint;
    userFeeIncrease?: number;
}

/**
 * IdentityKeyLimitsUpdate serialized as a plain object.
 */
export interface IdentityKeyLimitsUpdateObject {
    identityId: Uint8Array;
    nonce: bigint;
    keyId: number;
    totalBudget?: bigint;
    expiresAt?: bigint;
    userFeeIncrease: number;
    signature?: Uint8Array;
    signaturePublicKeyId?: number;
}

/**
 * IdentityKeyLimitsUpdate serialized as JSON.
 */
export interface IdentityKeyLimitsUpdateJSON {
    identityId: string;
    nonce: number | string;
    keyId: number;
    totalBudget?: number | string;
    expiresAt?: number | string;
    userFeeIncrease: number;
    signature?: string;
    signaturePublicKeyId?: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IdentityKeyLimitsUpdateTransitionOptions")]
    pub type IdentityKeyLimitsUpdateTransitionOptionsJs;

    #[wasm_bindgen(typescript_type = "IdentityKeyLimitsUpdateObject")]
    pub type IdentityKeyLimitsUpdateObjectJs;

    #[wasm_bindgen(typescript_type = "IdentityKeyLimitsUpdateJSON")]
    pub type IdentityKeyLimitsUpdateJSONJs;
}

/// Serde struct for IdentityKeyLimitsUpdateOptions (primitives only)
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdentityKeyLimitsUpdateOptionsInput {
    nonce: u64,
    key_id: u32,
    #[serde(default)]
    total_budget: Option<u64>,
    #[serde(default)]
    expires_at: Option<u64>,
    /// `undefined` reaches serde as a unit value, so the fee is read as an option and defaulted
    #[serde(default)]
    user_fee_increase: Option<UserFeeIncrease>,
}

#[wasm_bindgen(js_name = "IdentityKeyLimitsUpdate")]
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct IdentityKeyLimitsUpdateWasm(IdentityKeyLimitsUpdateTransition);

impl From<IdentityKeyLimitsUpdateTransition> for IdentityKeyLimitsUpdateWasm {
    fn from(val: IdentityKeyLimitsUpdateTransition) -> Self {
        IdentityKeyLimitsUpdateWasm(val)
    }
}

impl From<IdentityKeyLimitsUpdateWasm> for IdentityKeyLimitsUpdateTransition {
    fn from(val: IdentityKeyLimitsUpdateWasm) -> Self {
        val.0
    }
}

#[wasm_bindgen(js_class = IdentityKeyLimitsUpdate)]
impl IdentityKeyLimitsUpdateWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: IdentityKeyLimitsUpdateTransitionOptionsJs,
    ) -> WasmDppResult<IdentityKeyLimitsUpdateWasm> {
        // Extract complex types first (borrows &options)
        let identity_id: IdentifierWasm = try_from_options(&options, "identityId")?;

        // Deserialize primitive fields via serde last (consumes options)
        let input: IdentityKeyLimitsUpdateOptionsInput =
            serde_wasm_bindgen::from_value(options.into())
                .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        Ok(IdentityKeyLimitsUpdateWasm(
            IdentityKeyLimitsUpdateTransition::V0(IdentityKeyLimitsUpdateTransitionV0 {
                identity_id: identity_id.into(),
                nonce: input.nonce,
                key_id: input.key_id,
                total_budget: input.total_budget,
                expires_at: input.expires_at,
                user_fee_increase: input.user_fee_increase.unwrap_or_default(),
                signature_public_key_id: 0,
                signature: Default::default(),
            }),
        ))
    }

    #[wasm_bindgen(js_name = "toBytes")]
    pub fn to_bytes(&self) -> WasmDppResult<Vec<u8>> {
        Ok(self.0.serialize_to_bytes()?)
    }

    #[wasm_bindgen(js_name = "toHex")]
    pub fn to_hex(&self) -> WasmDppResult<String> {
        let bytes = self.0.serialize_to_bytes()?;
        Ok(encode(bytes.as_slice(), Hex))
    }

    #[wasm_bindgen(js_name = "toBase64")]
    pub fn to_base64(&self) -> WasmDppResult<String> {
        let bytes = self.0.serialize_to_bytes()?;
        Ok(encode(bytes.as_slice(), Base64))
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<IdentityKeyLimitsUpdateWasm> {
        let rs_transition =
            IdentityKeyLimitsUpdateTransition::deserialize_from_bytes_untrusted(bytes.as_slice())?;

        Ok(IdentityKeyLimitsUpdateWasm(rs_transition))
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> WasmDppResult<IdentityKeyLimitsUpdateWasm> {
        let bytes =
            decode(hex.as_str(), Hex).map_err(|e| WasmDppError::serialization(e.to_string()))?;
        IdentityKeyLimitsUpdateWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(base64: String) -> WasmDppResult<IdentityKeyLimitsUpdateWasm> {
        let bytes = decode(base64.as_str(), Base64)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        IdentityKeyLimitsUpdateWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(setter = "identityId")]
    pub fn set_identity_id(&mut self, identity_id: IdentifierLikeJs) -> WasmDppResult<()> {
        self.0.set_identity_id(identity_id.try_into()?);
        Ok(())
    }

    #[wasm_bindgen(setter = "nonce")]
    pub fn set_nonce(&mut self, nonce: &js_sys::BigInt) -> WasmDppResult<()> {
        self.0.set_nonce(try_to_u64(nonce, "nonce")?);
        Ok(())
    }

    #[wasm_bindgen(setter = "keyId")]
    pub fn set_key_id(&mut self, key_id: &js_sys::Number) -> WasmDppResult<()> {
        self.0.set_key_id(try_to_u32(key_id, "keyId")?);
        Ok(())
    }

    #[wasm_bindgen(setter = "totalBudget")]
    pub fn set_total_budget(&mut self, total_budget: Option<js_sys::BigInt>) -> WasmDppResult<()> {
        let total_budget = total_budget
            .map(|value| try_to_u64(&value, "totalBudget"))
            .transpose()?;
        self.0.set_total_budget(total_budget);
        Ok(())
    }

    #[wasm_bindgen(setter = "expiresAt")]
    pub fn set_expires_at(&mut self, expires_at: Option<js_sys::BigInt>) -> WasmDppResult<()> {
        let expires_at = expires_at
            .map(|value| try_to_u64(&value, "expiresAt"))
            .transpose()?;
        self.0.set_expires_at(expires_at);
        Ok(())
    }

    #[wasm_bindgen(setter = "signature")]
    pub fn set_signature(&mut self, signature: Vec<u8>) {
        self.0.set_signature_bytes(signature)
    }

    #[wasm_bindgen(setter = "signaturePublicKeyId")]
    pub fn set_signature_public_key_id(
        &mut self,
        #[wasm_bindgen(js_name = "publicKeyId")] public_key_id: &js_sys::Number,
    ) -> WasmDppResult<()> {
        self.0
            .set_signature_public_key_id(try_to_u32(public_key_id, "signaturePublicKeyId")?);
        Ok(())
    }

    #[wasm_bindgen(setter = "userFeeIncrease")]
    pub fn set_user_fee_increase(&mut self, amount: &js_sys::Number) -> WasmDppResult<()> {
        self.0
            .set_user_fee_increase(try_to_u16(amount, "userFeeIncrease")?);
        Ok(())
    }

    #[wasm_bindgen(getter = "signature")]
    pub fn signature(&self) -> Vec<u8> {
        self.0.signature().to_vec()
    }

    #[wasm_bindgen(getter = "signaturePublicKeyId")]
    pub fn signature_public_key_id(&self) -> u32 {
        self.0.signature_public_key_id()
    }

    #[wasm_bindgen(getter = "userFeeIncrease")]
    pub fn user_fee_increase(&self) -> u16 {
        self.0.user_fee_increase()
    }

    #[wasm_bindgen(getter = "identityId")]
    pub fn identity_id(&self) -> IdentifierWasm {
        self.0.identity_id().into()
    }

    #[wasm_bindgen(getter = "nonce")]
    pub fn nonce(&self) -> u64 {
        self.0.nonce()
    }

    #[wasm_bindgen(getter = "keyId")]
    pub fn key_id(&self) -> u32 {
        self.0.key_id()
    }

    #[wasm_bindgen(getter = "totalBudget")]
    pub fn total_budget(&self) -> Option<u64> {
        self.0.total_budget()
    }

    #[wasm_bindgen(getter = "expiresAt")]
    pub fn expires_at(&self) -> Option<u64> {
        self.0.expires_at()
    }

    #[wasm_bindgen(js_name = "toStateTransition")]
    pub fn to_state_transition(&self) -> StateTransitionWasm {
        StateTransitionWasm::from(StateTransition::from(self.0.clone()))
    }

    #[wasm_bindgen(js_name = "fromStateTransition")]
    pub fn from_state_transition(
        st: &StateTransitionWasm,
    ) -> WasmDppResult<IdentityKeyLimitsUpdateWasm> {
        let rs_st: StateTransition = st.clone().into();

        match rs_st {
            StateTransition::IdentityKeyLimitsUpdate(st) => Ok(IdentityKeyLimitsUpdateWasm(st)),
            _ => Err(WasmDppError::invalid_argument(
                "Invalid state transition type",
            )),
        }
    }
}

impl IdentityKeyLimitsUpdateWasm {
    pub fn set_signature_binary_data(&mut self, data: BinaryData) {
        self.0.set_signature(data)
    }
}

impl_wasm_conversions_inner!(
    IdentityKeyLimitsUpdateWasm,
    IdentityKeyLimitsUpdateTransition,
    IdentityKeyLimitsUpdate,
    IdentityKeyLimitsUpdateObjectJs,
    IdentityKeyLimitsUpdateJSONJs
);

impl_wasm_type_info!(IdentityKeyLimitsUpdateWasm, IdentityKeyLimitsUpdate);
