use crate::asset_lock_proof::AssetLockProofWasm;
use crate::enums::keys::purpose::PurposeWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::identity::transitions::public_key_in_creation::IdentityPublicKeyInCreationWasm;
use crate::impl_wasm_conversions;
use crate::impl_wasm_type_info;
use crate::state_transitions::StateTransitionWasm;
use crate::utils::{try_from_options, try_from_options_with, try_to_array, try_to_u32, try_to_u64};
use dpp::identity::KeyID;
use dpp::identity::state_transition::OptionallyAssetLockProved;
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::prelude::{IdentityNonce, Revision, UserFeeIncrease};
use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
use dpp::state_transition::identity_update_transition::accessors::IdentityUpdateTransitionAccessorsV0;
use dpp::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::state_transition::{
    StateTransition, StateTransitionIdentitySigned, StateTransitionLike,
    StateTransitionSingleSigned,
};
use serde::Deserialize;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const IDENTITY_UPDATE_OPTIONS_TS: &str = r#"
export interface IdentityUpdateTransitionOptions {
    identityId: IdentifierLike;
    revision: bigint;
    nonce: bigint;
    addPublicKeys: IdentityPublicKeyInCreation[];
    disablePublicKeys: number[];
    userFeeIncrease?: number;
}

/**
 * IdentityUpdateTransition serialized as a plain object.
 */
export interface IdentityUpdateTransitionObject {
    identityId: Uint8Array;
    revision: bigint;
    nonce: bigint;
    addPublicKeys: IdentityPublicKeyInCreationObject[];
    disablePublicKeys: number[];
    userFeeIncrease: number;
    signature?: Uint8Array;
    signaturePublicKeyId?: number;
}

/**
 * IdentityUpdateTransition serialized as JSON.
 */
export interface IdentityUpdateTransitionJSON {
    identityId: string;
    revision: string;
    nonce: string;
    addPublicKeys: IdentityPublicKeyInCreationJSON[];
    disablePublicKeys: number[];
    userFeeIncrease: number;
    signature?: string;
    signaturePublicKeyId?: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IdentityUpdateTransitionOptions")]
    pub type IdentityUpdateTransitionOptionsJs;

    #[wasm_bindgen(typescript_type = "IdentityUpdateTransitionObject")]
    pub type IdentityUpdateTransitionObjectJs;

    #[wasm_bindgen(typescript_type = "IdentityUpdateTransitionJSON")]
    pub type IdentityUpdateTransitionJSONJs;
}

/// Serde struct for IdentityUpdateTransitionOptions (primitives only)
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdentityUpdateTransitionOptionsInput {
    revision: Revision,
    nonce: IdentityNonce,
    #[serde(default)]
    user_fee_increase: UserFeeIncrease,
}

#[wasm_bindgen(js_name = "IdentityUpdateTransition")]
#[derive(Clone)]
pub struct IdentityUpdateTransitionWasm(IdentityUpdateTransition);

#[wasm_bindgen(js_class = IdentityUpdateTransition)]
impl IdentityUpdateTransitionWasm {
    #[wasm_bindgen(constructor)]
    pub fn constructor(
        options: IdentityUpdateTransitionOptionsJs,
    ) -> WasmDppResult<IdentityUpdateTransitionWasm> {
        // Extract complex types first (borrows &options)
        let identity_id: IdentifierWasm = try_from_options(&options, "identityId")?;

        let add_public_keys_array = try_from_options_with(&options, "addPublicKeys", |v| {
            try_to_array(v, "addPublicKeys")
        })?;
        let add_public_keys: Vec<IdentityPublicKeyInCreationWasm> =
            IdentityPublicKeyInCreationWasm::vec_from_array(&add_public_keys_array)?;

        let disable_public_keys_array =
            try_from_options_with(&options, "disablePublicKeys", |v| {
                try_to_array(v, "disablePublicKeys")
            })?;
        let disable_public_keys: Vec<KeyID> = disable_public_keys_array
            .iter()
            .enumerate()
            .map(|(i, v)| try_to_u32(&v, &format!("disablePublicKeys[{}]", i)))
            .collect::<WasmDppResult<Vec<KeyID>>>()?;

        // Deserialize primitive fields via serde last (consumes options)
        let input: IdentityUpdateTransitionOptionsInput =
            serde_wasm_bindgen::from_value(options.into())
                .map_err(|e| WasmDppError::invalid_argument(e.to_string()))?;

        Ok(IdentityUpdateTransitionWasm(IdentityUpdateTransition::V0(
            IdentityUpdateTransitionV0 {
                identity_id: identity_id.into(),
                revision: input.revision,
                nonce: input.nonce,
                add_public_keys: add_public_keys
                    .iter()
                    .map(|key| key.clone().into())
                    .collect(),
                disable_public_keys,
                user_fee_increase: input.user_fee_increase,
                signature_public_key_id: 0,
                signature: Default::default(),
            },
        )))
    }

    #[wasm_bindgen(getter = "revision")]
    pub fn revision(&self) -> Revision {
        self.0.revision()
    }

    #[wasm_bindgen(getter = "nonce")]
    pub fn nonce(&self) -> IdentityNonce {
        self.0.nonce()
    }

    #[wasm_bindgen(getter = "identityIdentifier")]
    pub fn identity_identifier(&self) -> IdentifierWasm {
        self.0.identity_id().into()
    }

    #[wasm_bindgen(getter = "purposeRequirement")]
    pub fn purpose_requirement(&self) -> Vec<String> {
        self.0
            .purpose_requirement()
            .iter()
            .map(|purpose| PurposeWasm::from(*purpose).into())
            .collect()
    }

    #[wasm_bindgen(getter = "modifiedDataIds")]
    pub fn modified_data_ids(&self) -> Vec<IdentifierWasm> {
        self.0
            .modified_data_ids()
            .iter()
            .map(|id| (*id).into())
            .collect()
    }

    #[wasm_bindgen(getter = "optionalAssetLockProof")]
    pub fn optional_asset_lock_proof(&self) -> Option<AssetLockProofWasm> {
        self.0
            .optional_asset_lock_proof()
            .map(|asset_lock| AssetLockProofWasm::from(asset_lock.clone()))
    }

    #[wasm_bindgen(getter = "publicKeyIdsToDisable")]
    pub fn public_key_ids_to_disable(&self) -> Vec<KeyID> {
        self.0.public_key_ids_to_disable().to_vec()
    }

    #[wasm_bindgen(getter = "publicKeyIdsToAdd")]
    pub fn public_key_ids_to_add(&self) -> Vec<IdentityPublicKeyInCreationWasm> {
        self.0
            .public_keys_to_add()
            .to_vec()
            .iter()
            .map(|id| id.clone().into())
            .collect()
    }

    #[wasm_bindgen(getter = "userFeeIncrease")]
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        self.0.user_fee_increase()
    }

    #[wasm_bindgen(setter = "revision")]
    pub fn set_revision(&mut self, revision: JsValue) -> WasmDppResult<()> {
        self.0.set_revision(try_to_u64(&revision, "revision")?);
        Ok(())
    }

    #[wasm_bindgen(setter = "nonce")]
    pub fn set_nonce(&mut self, nonce: JsValue) -> WasmDppResult<()> {
        self.0.set_nonce(try_to_u64(&nonce, "nonce")?);
        Ok(())
    }

    #[wasm_bindgen(setter = "identityIdentifier")]
    pub fn set_identity_identifier(
        &mut self,
        #[wasm_bindgen(js_name = "identityId")] identity_id: IdentifierLikeJs,
    ) -> WasmDppResult<()> {
        self.0.set_identity_id(identity_id.try_into()?);
        Ok(())
    }

    #[wasm_bindgen(setter = "publicKeyIdsToAdd")]
    pub fn set_public_key_ids_to_add(
        &mut self,
        #[wasm_bindgen(js_name = "addPublicKeys")] add_public_keys: &js_sys::Array,
    ) -> WasmDppResult<()> {
        let add_public_keys: Vec<IdentityPublicKeyInCreationWasm> =
            IdentityPublicKeyInCreationWasm::vec_from_array(add_public_keys)?;

        let keys: Vec<IdentityPublicKeyInCreation> =
            add_public_keys.iter().map(|id| id.clone().into()).collect();

        self.0.set_public_keys_to_add(keys);
        Ok(())
    }

    #[wasm_bindgen(setter = "publicKeyIdsToDisable")]
    pub fn set_public_key_ids_to_disable(
        &mut self,
        #[wasm_bindgen(js_name = "publicKeys")] public_keys: Vec<KeyID>,
    ) {
        self.0.set_public_key_ids_to_disable(public_keys)
    }

    #[wasm_bindgen(setter = "userFeeIncrease")]
    pub fn set_user_fee_increase(
        &mut self,
        #[wasm_bindgen(js_name = "userFeeIncrease")] user_fee_increase: UserFeeIncrease,
    ) {
        self.0.set_user_fee_increase(user_fee_increase)
    }

    #[wasm_bindgen(getter = "signature")]
    pub fn signature(&self) -> Vec<u8> {
        self.0.signature().to_vec()
    }

    #[wasm_bindgen(getter = "signaturePublicKeyId")]
    pub fn signature_public_key_id(&self) -> KeyID {
        self.0.signature_public_key_id()
    }

    #[wasm_bindgen(setter = "signature")]
    pub fn set_signature(&mut self, signature: Vec<u8>) {
        self.0.set_signature_bytes(signature)
    }

    #[wasm_bindgen(setter = "signaturePublicKeyId")]
    pub fn set_signature_public_key_id(
        &mut self,
        #[wasm_bindgen(js_name = "signaturePublicKeyId")] signature_public_key_id: KeyID,
    ) {
        self.0.set_signature_public_key_id(signature_public_key_id)
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> WasmDppResult<IdentityUpdateTransitionWasm> {
        let bytes =
            decode(hex.as_str(), Hex).map_err(|e| WasmDppError::serialization(e.to_string()))?;

        IdentityUpdateTransitionWasm::from_bytes(bytes)
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(base64: String) -> WasmDppResult<IdentityUpdateTransitionWasm> {
        let bytes = decode(base64.as_str(), Base64)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;

        IdentityUpdateTransitionWasm::from_bytes(bytes)
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
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<IdentityUpdateTransitionWasm> {
        let rs_transition = IdentityUpdateTransition::deserialize_from_bytes(bytes.as_slice())?;

        Ok(IdentityUpdateTransitionWasm(rs_transition))
    }

    #[wasm_bindgen(js_name = "toStateTransition")]
    pub fn to_state_transition(&self) -> StateTransitionWasm {
        StateTransitionWasm::from(StateTransition::from(self.0.clone()))
    }

    #[wasm_bindgen(js_name = "fromStateTransition")]
    pub fn from_state_transition(
        st: &StateTransitionWasm,
    ) -> WasmDppResult<IdentityUpdateTransitionWasm> {
        let rs_st: StateTransition = st.clone().into();

        match rs_st {
            StateTransition::IdentityUpdate(st) => Ok(IdentityUpdateTransitionWasm(st)),
            _ => Err(WasmDppError::invalid_argument(
                "Invalid state transition type",
            )),
        }
    }
}

impl_wasm_conversions!(
    IdentityUpdateTransitionWasm,
    IdentityUpdateTransition,
    IdentityUpdateTransitionObjectJs,
    IdentityUpdateTransitionJSONJs
);
impl_wasm_type_info!(IdentityUpdateTransitionWasm, IdentityUpdateTransition);
