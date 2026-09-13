use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::shielded::orchard_action::{SerializedOrchardActionWasm, actions_from_js_options};
use crate::state_transitions::StateTransitionWasm;
use crate::utils::try_vec_to_fixed_bytes;
use crate::utils::{try_from_options_optional_with, try_to_u16, try_to_u32, try_to_u64};
use crate::{impl_wasm_conversions_inner, impl_wasm_type_info};
use dpp::platform_value::BinaryData;
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::prelude::UserFeeIncrease;
use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
use dpp::state_transition::shield_from_identity_transition::ShieldFromIdentityTransition;
use dpp::state_transition::shield_from_identity_transition::accessors::ShieldFromIdentityTransitionAccessorsV0;
use dpp::state_transition::shield_from_identity_transition::v0::ShieldFromIdentityTransitionV0;
use dpp::state_transition::{
    StateTransition, StateTransitionHasUserFeeIncrease, StateTransitionIdentitySigned,
    StateTransitionLike, StateTransitionSingleSigned,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const TS_TYPES: &str = r#"
/**
 * Options for constructing a ShieldFromIdentityTransition (identity balance to shielded pool).
 * The bundle is an outputs-only Orchard bundle; the transition is identity-signed with a
 * TRANSFER key after construction.
 */
export interface ShieldFromIdentityTransitionOptions {
    identityId: IdentifierLike;
    amount: bigint;
    actions: SerializedOrchardAction[];
    anchor: Uint8Array;
    proof: Uint8Array;
    bindingSignature: Uint8Array;
    nonce: bigint;
    userFeeIncrease?: number;
}

/**
 * ShieldFromIdentityTransition serialized as a plain object.
 */
export interface ShieldFromIdentityTransitionObject {
    $formatVersion: string;
    identityId: Uint8Array;
    amount: bigint;
    actions: SerializedOrchardActionObject[];
    anchor: Uint8Array;
    proof: Uint8Array;
    bindingSignature: Uint8Array;
    nonce: bigint;
    userFeeIncrease: number;
    signaturePublicKeyId: number;
    signature: Uint8Array;
}

/**
 * ShieldFromIdentityTransition serialized as JSON (human-readable).
 */
export interface ShieldFromIdentityTransitionJSON {
    $formatVersion: string;
    identityId: string;
    amount: number | string;
    actions: SerializedOrchardActionJSON[];
    anchor: string;
    proof: string;
    bindingSignature: string;
    nonce: number | string;
    userFeeIncrease: number;
    signaturePublicKeyId: number;
    signature: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ShieldFromIdentityTransitionOptions")]
    pub type ShieldFromIdentityTransitionOptionsJs;

    #[wasm_bindgen(typescript_type = "ShieldFromIdentityTransitionObject")]
    pub type ShieldFromIdentityTransitionObjectJs;

    #[wasm_bindgen(typescript_type = "ShieldFromIdentityTransitionJSON")]
    pub type ShieldFromIdentityTransitionJSONJs;
}

/// Non-WASM-instance fields extracted from the constructor options via serde.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShieldFromIdentityTransitionSimpleFields {
    amount: u64,
    anchor: Vec<u8>,
    proof: Vec<u8>,
    binding_signature: Vec<u8>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(transparent)]
#[wasm_bindgen(js_name = ShieldFromIdentityTransition)]
pub struct ShieldFromIdentityTransitionWasm(ShieldFromIdentityTransition);

impl From<ShieldFromIdentityTransition> for ShieldFromIdentityTransitionWasm {
    fn from(v: ShieldFromIdentityTransition) -> Self {
        ShieldFromIdentityTransitionWasm(v)
    }
}

impl From<ShieldFromIdentityTransitionWasm> for ShieldFromIdentityTransition {
    fn from(v: ShieldFromIdentityTransitionWasm) -> Self {
        v.0
    }
}

#[wasm_bindgen(js_class = ShieldFromIdentityTransition)]
impl ShieldFromIdentityTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(
        options: ShieldFromIdentityTransitionOptionsJs,
    ) -> WasmDppResult<ShieldFromIdentityTransitionWasm> {
        let js_opts: &JsValue = options.as_ref();

        let identity_id: IdentifierWasm = crate::utils::try_from_options(&options, "identityId")?;
        let actions = actions_from_js_options(js_opts, "actions")?;
        let nonce: u64 =
            crate::utils::try_from_options_with(js_opts, "nonce", |v| try_to_u64(v, "nonce"))?;
        let user_fee_increase: UserFeeIncrease =
            try_from_options_optional_with(js_opts, "userFeeIncrease", |v| {
                try_to_u16(v, "userFeeIncrease")
            })?
            .unwrap_or(0);

        let fields: ShieldFromIdentityTransitionSimpleFields =
            serde_wasm_bindgen::from_value(options.into())
                .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        let anchor: [u8; 32] = try_vec_to_fixed_bytes(fields.anchor, "anchor")?;
        let binding_signature: [u8; 64] =
            try_vec_to_fixed_bytes(fields.binding_signature, "bindingSignature")?;

        Ok(ShieldFromIdentityTransitionWasm(
            ShieldFromIdentityTransition::V0(ShieldFromIdentityTransitionV0 {
                identity_id: identity_id.into(),
                amount: fields.amount,
                actions: actions.into_iter().map(Into::into).collect(),
                anchor,
                proof: fields.proof,
                binding_signature,
                nonce,
                user_fee_increase,
                signature_public_key_id: 0,
                signature: Default::default(),
            }),
        ))
    }

    /// The identity whose balance funds the shield.
    #[wasm_bindgen(getter = "identityId")]
    pub fn identity_id(&self) -> IdentifierWasm {
        self.0.identity_id().into()
    }

    #[wasm_bindgen(setter = "identityId")]
    pub fn set_identity_id(&mut self, identity_id: IdentifierLikeJs) -> WasmDppResult<()> {
        self.0.set_identity_id(identity_id.try_into()?);
        Ok(())
    }

    /// Credits leaving the identity balance and entering the pool.
    #[wasm_bindgen(getter = "amount")]
    pub fn amount(&self) -> u64 {
        self.0.amount()
    }

    /// Returns the serialized Orchard actions.
    #[wasm_bindgen(getter = "actions")]
    pub fn actions(&self) -> Vec<SerializedOrchardActionWasm> {
        self.0
            .actions()
            .iter()
            .cloned()
            .map(SerializedOrchardActionWasm::from)
            .collect()
    }

    /// Returns the anchor (32-byte Merkle root).
    #[wasm_bindgen(getter = "anchor")]
    pub fn anchor(&self) -> Vec<u8> {
        self.0.anchor().to_vec()
    }

    /// Returns the Halo2 proof bytes.
    #[wasm_bindgen(getter = "proof")]
    pub fn proof(&self) -> Vec<u8> {
        self.0.proof().to_vec()
    }

    /// Returns the RedPallas binding signature (64 bytes).
    #[wasm_bindgen(getter = "bindingSignature")]
    pub fn binding_signature(&self) -> Vec<u8> {
        self.0.binding_signature().to_vec()
    }

    #[wasm_bindgen(getter = "nonce")]
    pub fn nonce(&self) -> u64 {
        self.0.nonce()
    }

    #[wasm_bindgen(setter = "nonce")]
    pub fn set_nonce(&mut self, nonce: &js_sys::BigInt) -> WasmDppResult<()> {
        self.0.set_nonce(try_to_u64(nonce, "nonce")?);
        Ok(())
    }

    #[wasm_bindgen(getter = "userFeeIncrease")]
    pub fn user_fee_increase(&self) -> u16 {
        self.0.user_fee_increase()
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

    #[wasm_bindgen(setter = "signature")]
    pub fn set_signature(&mut self, signature: Vec<u8>) {
        self.0.set_signature_bytes(signature)
    }

    #[wasm_bindgen(getter = "signaturePublicKeyId")]
    pub fn signature_public_key_id(&self) -> u32 {
        self.0.signature_public_key_id()
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
            &StateTransition::ShieldFromIdentity(self.0.clone()),
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
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<ShieldFromIdentityTransitionWasm> {
        let st = StateTransition::deserialize_from_bytes(&bytes)?;
        match st {
            StateTransition::ShieldFromIdentity(inner) => Ok(inner.into()),
            _ => Err(WasmDppError::invalid_argument(
                "Invalid state transition type: expected ShieldFromIdentity",
            )),
        }
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> WasmDppResult<ShieldFromIdentityTransitionWasm> {
        let bytes =
            decode(hex.as_str(), Hex).map_err(|e| WasmDppError::serialization(e.to_string()))?;
        ShieldFromIdentityTransitionWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(base64: String) -> WasmDppResult<ShieldFromIdentityTransitionWasm> {
        let bytes = decode(base64.as_str(), Base64)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;
        ShieldFromIdentityTransitionWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = toStateTransition)]
    pub fn to_state_transition(&self) -> StateTransitionWasm {
        StateTransition::ShieldFromIdentity(self.0.clone()).into()
    }

    #[wasm_bindgen(js_name = "fromStateTransition")]
    pub fn from_state_transition(
        st: &StateTransitionWasm,
    ) -> WasmDppResult<ShieldFromIdentityTransitionWasm> {
        let rs_st: StateTransition = st.clone().into();
        match rs_st {
            StateTransition::ShieldFromIdentity(inner) => Ok(inner.into()),
            _ => Err(WasmDppError::invalid_argument(
                "Invalid state transition type: expected ShieldFromIdentity",
            )),
        }
    }
}

impl ShieldFromIdentityTransitionWasm {
    pub fn set_signature_binary_data(&mut self, data: BinaryData) {
        self.0.set_signature(data)
    }
}

impl_wasm_conversions_inner!(
    ShieldFromIdentityTransitionWasm,
    ShieldFromIdentityTransition,
    ShieldFromIdentityTransition,
    ShieldFromIdentityTransitionObjectJs,
    ShieldFromIdentityTransitionJSONJs
);

impl_wasm_type_info!(
    ShieldFromIdentityTransitionWasm,
    ShieldFromIdentityTransition
);
