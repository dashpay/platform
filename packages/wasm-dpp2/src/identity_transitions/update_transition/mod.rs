use crate::asset_lock_proof::AssetLockProofWasm;
use crate::enums::keys::purpose::PurposeWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::IdentifierWasm;
use crate::identity_transitions::public_key_in_creation::IdentityPublicKeyInCreationWasm;
use crate::state_transition::StateTransitionWasm;
use dpp::identity::KeyID;
use dpp::identity::state_transition::OptionallyAssetLockProved;
use dpp::platform_value::string_encoding::Encoding::{Base64, Hex};
use dpp::platform_value::string_encoding::{decode, encode};
use dpp::prelude::{IdentityNonce, Revision, UserFeeIncrease};
use dpp::serialization::{PlatformDeserializable, PlatformSerializable, Signable};
use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
use dpp::state_transition::identity_update_transition::accessors::IdentityUpdateTransitionAccessorsV0;
use dpp::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::state_transition::{StateTransition, StateTransitionIdentitySigned, StateTransitionLike};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(js_name = "IdentityUpdateTransition")]
#[derive(Clone)]
pub struct IdentityUpdateTransitionWasm(IdentityUpdateTransition);

#[wasm_bindgen(js_class = IdentityUpdateTransition)]
impl IdentityUpdateTransitionWasm {
    #[wasm_bindgen(getter = __type)]
    pub fn type_name(&self) -> String {
        "IdentityUpdateTransition".to_string()
    }

    #[wasm_bindgen(getter = __struct)]
    pub fn struct_name() -> String {
        "IdentityUpdateTransition".to_string()
    }

    #[wasm_bindgen(constructor)]
    pub fn new(
        js_identity_id: &JsValue,
        revision: Revision,
        nonce: IdentityNonce,
        js_add_public_keys: &js_sys::Array,
        disable_public_keys: Vec<KeyID>,
        user_fee_increase: Option<UserFeeIncrease>,
    ) -> WasmDppResult<IdentityUpdateTransitionWasm> {
        let identity_id = IdentifierWasm::try_from(js_identity_id)?;

        let add_public_keys: Vec<IdentityPublicKeyInCreationWasm> =
            IdentityPublicKeyInCreationWasm::vec_from_js_value(js_add_public_keys)?;

        Ok(IdentityUpdateTransitionWasm(IdentityUpdateTransition::V0(
            IdentityUpdateTransitionV0 {
                identity_id: identity_id.into(),
                revision,
                nonce,
                add_public_keys: add_public_keys
                    .clone()
                    .iter()
                    .map(|key| key.clone().into())
                    .collect(),
                disable_public_keys,
                user_fee_increase: user_fee_increase.unwrap_or(0),
                signature_public_key_id: 0,
                signature: Default::default(),
            },
        )))
    }

    #[wasm_bindgen(getter = "revision")]
    pub fn get_revision(&self) -> Revision {
        self.0.revision()
    }

    #[wasm_bindgen(getter = "nonce")]
    pub fn get_nonce(&self) -> IdentityNonce {
        self.0.nonce()
    }

    #[wasm_bindgen(getter = "identityIdentifier")]
    pub fn get_identity_identifier(&self) -> IdentifierWasm {
        self.0.identity_id().into()
    }

    #[wasm_bindgen(js_name = "getPurposeRequirement")]
    pub fn get_purpose_requirement(&self) -> Vec<String> {
        self.0
            .purpose_requirement()
            .iter()
            .map(|purpose| PurposeWasm::from(purpose.clone()).into())
            .collect()
    }

    #[wasm_bindgen(js_name = "getModifiedDataIds")]
    pub fn get_modified_data_ids(&self) -> Vec<IdentifierWasm> {
        self.0
            .modified_data_ids()
            .iter()
            .map(|id| id.clone().into())
            .collect()
    }

    #[wasm_bindgen(js_name = "getOptionalAssetLockProof")]
    pub fn get_optional_asset_lock_proof(&self) -> JsValue {
        match self.0.optional_asset_lock_proof() {
            Some(asset_lock) => JsValue::from(AssetLockProofWasm::from(asset_lock.clone())),
            None => JsValue::null(),
        }
    }

    #[wasm_bindgen(getter = "publicKeyIdsToDisable")]
    pub fn get_public_key_ids_to_disable(&self) -> Vec<KeyID> {
        self.0.public_key_ids_to_disable().to_vec()
    }

    #[wasm_bindgen(getter = "publicKeyIdsToAdd")]
    pub fn get_public_key_ids_to_add(&self) -> Vec<IdentityPublicKeyInCreationWasm> {
        self.0
            .public_keys_to_add()
            .to_vec()
            .iter()
            .map(|id| id.clone().into())
            .collect()
    }

    #[wasm_bindgen(getter = "userFeeIncrease")]
    pub fn get_user_fee_increase(&self) -> UserFeeIncrease {
        self.0.user_fee_increase()
    }

    #[wasm_bindgen(setter = "revision")]
    pub fn set_revision(&mut self, revision: Revision) {
        self.0.set_revision(revision);
    }

    #[wasm_bindgen(setter = "nonce")]
    pub fn set_nonce(&mut self, nonce: IdentityNonce) {
        self.0.set_nonce(nonce);
    }

    #[wasm_bindgen(setter = "identityIdentifier")]
    pub fn set_identity_identifier(&mut self, js_identity_id: &JsValue) -> WasmDppResult<()> {
        let identity_id = IdentifierWasm::try_from(js_identity_id)?;
        self.0.set_identity_id(identity_id.clone().into());
        Ok(())
    }

    #[wasm_bindgen(setter = "publicKeyIdsToAdd")]
    pub fn set_public_key_ids_to_add(
        &mut self,
        js_add_public_keys: &js_sys::Array,
    ) -> WasmDppResult<()> {
        let add_public_keys: Vec<IdentityPublicKeyInCreationWasm> =
            IdentityPublicKeyInCreationWasm::vec_from_js_value(js_add_public_keys)?;

        let keys: Vec<IdentityPublicKeyInCreation> =
            add_public_keys.iter().map(|id| id.clone().into()).collect();

        self.0.set_public_keys_to_add(keys);
        Ok(())
    }

    #[wasm_bindgen(setter = "publicKeyIdsToDisable")]
    pub fn set_public_key_ids_to_disable(&mut self, public_keys: Vec<KeyID>) {
        self.0.set_public_key_ids_to_disable(public_keys)
    }

    #[wasm_bindgen(setter = "userFeeIncrease")]
    pub fn set_user_fee_increase(&mut self, user_fee_increase: UserFeeIncrease) {
        self.0.set_user_fee_increase(user_fee_increase)
    }

    #[wasm_bindgen(getter = "signature")]
    pub fn get_signature(&self) -> Vec<u8> {
        self.0.signature().to_vec()
    }

    #[wasm_bindgen(js_name = "getSignableBytes")]
    pub fn get_signable_bytes(&self) -> WasmDppResult<Vec<u8>> {
        Ok(self.0.signable_bytes()?)
    }

    #[wasm_bindgen(getter = "signaturePublicKeyId")]
    pub fn get_signature_public_key_id(&self) -> KeyID {
        self.0.signature_public_key_id()
    }

    #[wasm_bindgen(setter = "signature")]
    pub fn set_signature(&mut self, signature: Vec<u8>) {
        self.0.set_signature_bytes(signature)
    }

    #[wasm_bindgen(setter = "signaturePublicKeyId")]
    pub fn set_signature_public_key_id(&mut self, signature_public_key_id: KeyID) {
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

    #[wasm_bindgen(js_name = "base64")]
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
