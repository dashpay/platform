use dpp::dashcore::anyhow::Context;
use dpp::identifier::Identifier;
use js_sys::Array;
use js_sys::ArrayIter;
use js_sys::Function;
use serde_json::Value;
use wasm_bindgen::convert::IntoWasmAbi;
use wasm_bindgen::prelude::*;

use dpp::identity::state_transition::asset_lock_proof::AssetLockProof;
use dpp::identity::IdentityPublicKey;
use dpp::identity::{Identity, KeyID};
use dpp::metadata::Metadata;
use dpp::util::json_value::JsonValueExt;
use dpp::{ProtocolError, SerdeParsingError};
use web_sys::console::log_1;
use web_sys::console::log_2;

use core::iter::FromIterator;
use std::borrow::Borrow;
use std::convert::TryInto;

use dpp::util::string_encoding::Encoding;

use serde::{Deserialize, Serialize};

use crate::errors::from_dpp_err;
use crate::identifier::IdentifierWrapper;
use crate::utils;
use crate::utils::into_vec;
use crate::utils::to_vec_of_serde_values;
use crate::utils::ToSerdeJSONExt;
use crate::IdentityPublicKeyWasm;
use crate::{identity_public_key, MetadataWasm};

#[wasm_bindgen(js_name=Identity)]
pub struct IdentityWasm(Identity);

#[wasm_bindgen(js_name=AssetLockProof)]
pub struct AssetLockProofWasm(AssetLockProof);
impl From<AssetLockProof> for AssetLockProofWasm {
    fn from(v: AssetLockProof) -> Self {
        AssetLockProofWasm(v)
    }
}

#[wasm_bindgen(js_class=Identity)]
impl IdentityWasm {
    #[wasm_bindgen(js_name=doStuff)]
    pub fn do_stuff(raw_identity: JsValue) -> Result<js_sys::JsString, JsValue> {
        js_sys::JSON::stringify(&raw_identity)
    }

    #[wasm_bindgen(constructor)]
    pub fn new(raw_identity: JsValue) -> Result<IdentityWasm, JsValue> {
        let identity_json = utils::stringify(&raw_identity)?;
        let raw_identity: Value =
            serde_json::from_str(&identity_json).map_err(|e| e.to_string())?;

        let identity = Identity::from_raw_identity(raw_identity).unwrap();
        Ok(IdentityWasm(identity))
    }

    #[wasm_bindgen(js_name=getProtocolVersion)]
    pub fn get_protocol_version(&self) -> u32 {
        self.0.get_protocol_version()
    }

    #[wasm_bindgen(js_name=getId)]
    pub fn get_id(&self) -> IdentifierWrapper {
        self.0.get_id().clone().into()
    }

    // TODO: There's a problem here - if the value is not a vec, this method just won't return
    #[wasm_bindgen(js_name=setPublicKeys)]
    pub fn set_public_keys(&mut self, public_keys: js_sys::Array) -> Result<usize, JsValue> {
        let raw_public_keys = to_vec_of_serde_values(public_keys.iter())?;
        if raw_public_keys.is_empty() {
            return Err(format!("Setting public keys failed. The input ('{}') is invalid. You must use array of PublicKeys", public_keys.to_string()).into());
        }

        let public_keys = raw_public_keys
            .into_iter()
            .map(IdentityPublicKey::from_raw_object)
            .collect::<Result<_, _>>()
            .map_err(|e| {
                format!(
                    "converting to collection of IdentityPublicKeys failed: {:#}",
                    e
                )
            })?;

        self.0.set_public_keys(public_keys);

        Ok(self.0.get_public_keys().len())
    }

    #[wasm_bindgen(js_name=getPublicKeys)]
    pub fn get_public_keys(&self) -> Vec<JsValue> {
        self.0
            .get_public_keys()
            .iter()
            .map(IdentityPublicKey::to_owned)
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
    pub fn set_asset_lock_proof(&mut self, lock: JsValue) {
        self.0
            .set_asset_lock_proof(JsValue::into_serde(&lock).unwrap());
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
    pub fn set_metadata(&mut self, metadata: MetadataWasm) {
        self.0.set_metadata(metadata.into());
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
        let pks = self
            .0
            .public_keys
            .iter()
            .map(|pk| pk.to_json())
            .collect::<Result<Vec<serde_json::Value>, SerdeParsingError>>()
            .map_err(|e| from_dpp_err(e.into()))?;
        let mut identity_json =
            serde_json::to_value(self.0.clone()).map_err(|e| from_dpp_err(e.into()))?;

        let map = identity_json.as_object_mut().ok_or_else(|| {
            from_dpp_err(ProtocolError::Generic(
                "Expect identity to be a json map".into(),
            ))
        })?;
        map.insert("publicKeys".into(), serde_json::Value::from(pks));

        let identity_json_string =
            serde_json::to_string(&identity_json).map_err(|e| from_dpp_err(e.into()))?;
        js_sys::JSON::parse(&identity_json_string)
    }

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(&self, some_option: Option<bool>) -> Result<JsValue, JsValue> {
        let pks = self
            .0
            .public_keys
            .iter()
            .map(|pk| pk.to_raw_json_object(some_option.unwrap_or(false)))
            .collect::<Result<Vec<serde_json::Value>, SerdeParsingError>>()
            .map_err(|e| from_dpp_err(e.into()))?;
        let mut identity_json =
            serde_json::to_value(self.0.clone()).map_err(|e| from_dpp_err(e.into()))?;

        let map = identity_json.as_object_mut().ok_or_else(|| {
            from_dpp_err(ProtocolError::Generic(
                "Expect identity to be a json map".into(),
            ))
        })?;

        map.insert(
            "id".into(),
            serde_json::Value::from(self.0.id.buffer.to_vec()),
        );
        map.insert("publicKeys".into(), serde_json::Value::from(pks));

        let identity_json_string =
            serde_json::to_string(&identity_json).map_err(|e| from_dpp_err(e.into()))?;
        js_sys::JSON::parse(&identity_json_string)
    }

    #[wasm_bindgen(js_name=toString)]
    pub fn to_string(&self) -> String {
        serde_json::to_string(&self.0).unwrap()
    }

    #[wasm_bindgen(js_name=toBuffer)]
    pub fn to_buffer(&self) -> Vec<u8> {
        self.0.to_buffer().unwrap()
    }

    #[wasm_bindgen]
    pub fn hash(&self) -> Result<Vec<u8>, JsValue> {
        self.0.hash().map_err(from_dpp_err)
    }

    #[wasm_bindgen(js_name=addPublicKey)]
    pub fn add_public_key(&mut self, public_key: IdentityPublicKeyWasm) {
        self.0.public_keys.push(public_key.into());
    }

    // The method `addPublicKeys()` takes an array of `IdentityPublicKeyWasm` as an input. But elements of the array
    // are available ONLY as `JsValue`. WASM-bindgen uses output from `toJSON()` to store WASM-object as `JsValue`.
    // `toJSON()` converts binary data to `base64` or `base58`. Therefore we need to use `from_json_object()` constructor to
    // to convert strings back into bytes and get `IdentityPublicKeyWasm`
    #[wasm_bindgen(js_name=addPublicKeys)]
    pub fn add_public_keys(&mut self, js_public_keys: js_sys::Array) -> Result<(), JsValue> {
        let json_objects = to_vec_of_serde_values(js_public_keys.iter())?;

        let public_keys: Vec<IdentityPublicKey> = json_objects
            .into_iter()
            .map(IdentityPublicKey::from_json_object)
            .collect::<Result<Vec<IdentityPublicKey>, ProtocolError>>()
            .map_err(from_dpp_err)?;

        self.0
            .add_public_keys(public_keys.into_iter().map(Into::into));
        Ok(())
    }

    #[wasm_bindgen(js_name=getPublicKeyMaxId)]
    pub fn get_public_ke_max_id(&self) -> f64 {
        self.0.get_public_key_max_id() as f64
    }
}
