mod factory_utils;
pub mod identity_facade;
mod identity_factory;
mod identity_public_key;
mod validation;

use js_sys::Array;
use serde_json::Value;
use wasm_bindgen::prelude::*;

use dpp::identity::state_transition::asset_lock_proof::AssetLockProof;
use dpp::identity::IdentityPublicKey;
use dpp::identity::{Identity, KeyID};
use dpp::metadata::Metadata;
use dpp::serialization::serialization_traits::{PlatformDeserializable, PlatformSerializable};
use dpp::{Convertible, ProtocolError};

use crate::identifier::IdentifierWrapper;
use crate::utils::{to_vec_of_serde_values, IntoWasm, WithJsError};
use crate::MetadataWasm;
use crate::{utils, with_js_error};
pub use identity_public_key::*;

use crate::buffer::Buffer;
use crate::errors::from_dpp_err;
pub use state_transition::*;

pub mod credits_converter;
pub mod errors;
pub mod state_transition;

#[wasm_bindgen(js_name=Identity)]
#[derive(Clone)]
pub struct IdentityWasm(Identity);

impl From<IdentityWasm> for Identity {
    fn from(identity: IdentityWasm) -> Self {
        identity.0
    }
}

impl From<Identity> for IdentityWasm {
    fn from(identity: Identity) -> Self {
        Self(identity)
    }
}

#[wasm_bindgen(js_class=Identity)]
impl IdentityWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(raw_identity: JsValue) -> Result<IdentityWasm, JsValue> {
        let identity_json = utils::stringify(&raw_identity)?;
        let raw_identity: Value =
            serde_json::from_str(&identity_json).map_err(|e| e.to_string())?;

        let identity = Identity::from_json(raw_identity).map_err(from_dpp_err)?;
        Ok(IdentityWasm(identity))
    }

    #[wasm_bindgen(js_name=getProtocolVersion)]
    pub fn get_protocol_version(&self) -> u32 {
        self.0.get_feature_version()
    }

    #[wasm_bindgen(js_name=getId)]
    pub fn get_id(&self) -> IdentifierWrapper {
        (*self.0.get_id()).into()
    }

    #[wasm_bindgen(js_name=setPublicKeys)]
    pub fn set_public_keys(&mut self, public_keys: js_sys::Array) -> Result<usize, JsValue> {
        let raw_public_keys = to_vec_of_serde_values(public_keys.iter())?;
        if raw_public_keys.is_empty() {
            return Err(format!("Setting public keys failed. The input ('{}') is invalid. You must use array of PublicKeys", public_keys.to_string()).into());
        }

        let public_keys = raw_public_keys
            .into_iter()
            .map(|v| IdentityPublicKey::from_json_object(v).map(|key| (key.id, key)))
            .collect::<Result<_, _>>()
            .map_err(|e| format!("converting to collection of IdentityPublicKeys failed: {e:#}"))?;

        self.0.set_public_keys(public_keys);

        Ok(self.0.public_keys().len())
    }

    #[wasm_bindgen(js_name=getPublicKeys)]
    pub fn get_public_keys(&self) -> Vec<JsValue> {
        self.0
            .public_keys()
            .iter()
            .map(|(_, k)| k.to_owned())
            .map(IdentityPublicKeyWasm::from)
            .map(JsValue::from)
            .collect()
    }

    #[wasm_bindgen(js_name=getPublicKeyById)]
    pub fn get_public_key_by_id(&self, key_id: u32) -> Option<IdentityPublicKeyWasm> {
        let key_id = key_id as KeyID;
        self.0
            .get_public_key_by_id(key_id)
            .map(IdentityPublicKey::to_owned)
            .map(Into::into)
    }

    #[wasm_bindgen(getter)]
    pub fn balance(&self) -> f64 {
        self.0.get_balance() as f64
    }

    #[wasm_bindgen(js_name=getBalance)]
    pub fn get_balance(&self) -> f64 {
        self.0.get_balance() as f64
    }

    #[wasm_bindgen(js_name=setBalance)]
    pub fn set_balance(&mut self, balance: f64) {
        self.0.set_balance(balance as u64);
    }

    #[wasm_bindgen(js_name=increaseBalance)]
    pub fn increase_balance(&mut self, amount: f64) -> f64 {
        self.0.increase_balance(amount as u64) as f64
    }

    #[wasm_bindgen(js_name=reduceBalance)]
    pub fn reduce_balance(&mut self, amount: f64) -> f64 {
        self.0.reduce_balance(amount as u64) as f64
    }

    #[wasm_bindgen(js_name=setAssetLockProof)]
    pub fn set_asset_lock_proof(&mut self, lock: JsValue) -> Result<(), JsValue> {
        let asset_lock_proof = create_asset_lock_proof_from_wasm_instance(&lock)?;
        self.0.set_asset_lock_proof(asset_lock_proof);
        Ok(())
    }

    #[wasm_bindgen(js_name=getAssetLockProof)]
    pub fn get_asset_lock_proof(&self) -> Option<AssetLockProofWasm> {
        self.0
            .get_asset_lock_proof()
            .map(AssetLockProof::to_owned)
            .map(Into::into)
    }

    #[wasm_bindgen(js_name=setRevision)]
    pub fn set_revision(&mut self, revision: f64) {
        self.0.set_revision(revision as u64);
    }

    #[wasm_bindgen(js_name=getRevision)]
    pub fn get_revision(&self) -> f64 {
        self.0.get_revision() as f64
    }

    #[wasm_bindgen(js_name=setMetadata)]
    pub fn set_metadata(&mut self, metadata: JsValue) -> Result<(), JsValue> {
        if !metadata.is_falsy() {
            let metadata = metadata.to_wasm::<MetadataWasm>("Metadata")?;
            self.0.set_metadata(metadata.to_owned().into())
        }

        Ok(())
    }

    #[wasm_bindgen(js_name=getMetadata)]
    pub fn get_metadata(&self) -> Option<MetadataWasm> {
        self.0
            .get_metadata()
            .map(Metadata::to_owned)
            .map(Into::into)
    }

    #[wasm_bindgen(js_name=from)]
    pub fn from(object: JsValue) -> Self {
        let i: Identity = serde_json::from_str(&object.as_string().unwrap()).unwrap();
        IdentityWasm(i)
    }

    #[wasm_bindgen(js_name=toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let json = self.0.to_json().with_js_error()?;
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        with_js_error!(json.serialize(&serializer))
    }

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsValue> {
        let js_public_keys = js_sys::Array::new();
        for pk in self.0.public_keys().values() {
            let pk_wasm = IdentityPublicKeyWasm::from(pk.to_owned());
            js_public_keys.push(&pk_wasm.to_object()?);
        }

        let value = self.0.to_cleaned_object().with_js_error()?;

        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        let js_object = with_js_error!(value.serialize(&serializer))?;

        let id = Buffer::from_bytes(self.0.id.as_slice());

        js_sys::Reflect::set(&js_object, &"id".to_owned().into(), &id)?;

        js_sys::Reflect::set(
            &js_object,
            &"publicKeys".to_owned().into(),
            &JsValue::from(&js_public_keys),
        )?;

        Ok(js_object)
    }

    #[wasm_bindgen(js_name=toBuffer)]
    pub fn to_buffer(&self) -> Result<Buffer, JsValue> {
        let bytes = PlatformSerializable::serialize(&self.0.clone()).with_js_error()?;
        Ok(Buffer::from_bytes(&bytes))
    }

    #[wasm_bindgen]
    pub fn hash(&self) -> Result<Vec<u8>, JsValue> {
        self.0.hash().with_js_error()
    }

    #[wasm_bindgen(js_name=addPublicKey)]
    pub fn add_public_key(&mut self, public_key: IdentityPublicKeyWasm) {
        self.0
            .public_keys()
            .insert(public_key.get_id(), public_key.into());
    }

    // The method `addPublicKeys()` takes an variadic array of `IdentityPublicKeyWasm` as an input. But elements of the array
    // are available ONLY as `JsValue`. WASM-bindgen uses output from `toJSON()` to store WASM-object as `JsValue`.
    // `toJSON()` converts binary data to `base64` or `base58`. Therefore we need to use `from_json_object()` constructor to
    // to convert strings back into bytes and get `IdentityPublicKeyWasm`
    #[wasm_bindgen(js_name=addPublicKeys, variadic)]
    pub fn add_public_keys(&mut self, js_public_keys: JsValue) -> Result<(), JsValue> {
        let js_public_keys_array = Array::from(&js_public_keys);
        let json_objects = to_vec_of_serde_values(js_public_keys_array.iter())?;

        let public_keys: Vec<IdentityPublicKey> = json_objects
            .into_iter()
            .map(IdentityPublicKey::from_json_object)
            .collect::<Result<Vec<IdentityPublicKey>, ProtocolError>>()
            .with_js_error()?;

        self.0
            .add_public_keys(public_keys.into_iter().map(Into::into));
        Ok(())
    }

    #[wasm_bindgen(js_name=getPublicKeyMaxId)]
    pub fn get_public_ke_max_id(&self) -> f64 {
        self.0.get_public_key_max_id() as f64
    }

    #[wasm_bindgen(js_name=fromBuffer)]
    pub fn from_buffer(buffer: Vec<u8>) -> Result<IdentityWasm, JsValue> {
        let identity: Identity =
            PlatformDeserializable::deserialize(buffer.as_slice()).with_js_error()?;
        Ok(identity.into())
    }
}
