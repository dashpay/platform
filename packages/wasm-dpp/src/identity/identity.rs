use crate::buffer::Buffer;
use crate::errors::from_dpp_err;
use crate::identifier::IdentifierWrapper;
use crate::identity::IdentityPublicKeyWasm;
use crate::metadata::MetadataWasm;
use crate::utils::{Inner, IntoWasm, WithJsError};
use crate::with_js_error;
use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{Identity, IdentityPublicKey, KeyID};
use dpp::metadata::Metadata;
use dpp::platform_value::ReplacementType;
use dpp::serialization::PlatformDeserializable;
use dpp::serialization::PlatformSerializable;
use dpp::serialization::ValueConvertible;
use dpp::version::PlatformVersion;
use serde::Serialize;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name=Identity)]
#[derive(Clone)]
pub struct IdentityWasm {
    inner: Identity,
    metadata: Option<Metadata>,
}

impl From<IdentityWasm> for Identity {
    fn from(identity: IdentityWasm) -> Self {
        identity.inner
    }
}

impl From<Identity> for IdentityWasm {
    fn from(identity: Identity) -> Self {
        Self {
            inner: identity,
            metadata: None,
        }
    }
}
#[wasm_bindgen(js_class=Identity)]
impl IdentityWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(platform_version: u32) -> Result<IdentityWasm, JsValue> {
        let platform_version =
            &PlatformVersion::get(platform_version).map_err(|e| JsValue::from(e.to_string()))?;

        Identity::default_versioned(platform_version)
            .map(Into::into)
            .map_err(from_dpp_err)
    }

    #[wasm_bindgen(js_name=getId)]
    pub fn get_id(&self) -> IdentifierWrapper {
        self.inner.id().into()
    }

    #[wasm_bindgen(js_name=setId)]
    pub fn set_id(&mut self, id: IdentifierWrapper) {
        self.inner.set_id(id.into());
    }

    #[wasm_bindgen(js_name=setPublicKeys)]
    pub fn set_public_keys(&mut self, public_keys: js_sys::Array) -> Result<usize, JsValue> {
        if public_keys.length() == 0 {
            return Err(format!("Setting public keys failed. The input ('{}') is invalid. You must use array of PublicKeys", public_keys.to_string()).into());
        }

        let public_keys = public_keys
            .iter()
            .map(|key| {
                key.to_wasm::<IdentityPublicKeyWasm>("IdentityPublicKey")
                    .map(|key| {
                        let key = IdentityPublicKey::from(key.to_owned());
                        (key.id(), key)
                    })
            })
            .collect::<Result<_, _>>()?;

        self.inner.set_public_keys(public_keys);

        Ok(self.inner.public_keys().len())
    }

    #[wasm_bindgen(js_name=getPublicKeys)]
    pub fn get_public_keys(&self) -> Vec<JsValue> {
        self.inner
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
        self.inner
            .get_public_key_by_id(key_id)
            .map(IdentityPublicKey::to_owned)
            .map(Into::into)
    }

    #[wasm_bindgen(getter)]
    pub fn balance(&self) -> f64 {
        self.inner.balance() as f64
    }

    #[wasm_bindgen(js_name=getBalance)]
    pub fn get_balance(&self) -> f64 {
        self.inner.balance() as f64
    }

    #[wasm_bindgen(js_name=setBalance)]
    pub fn set_balance(&mut self, balance: f64) {
        self.inner.set_balance(balance as u64);
    }

    #[wasm_bindgen(js_name=increaseBalance)]
    pub fn increase_balance(&mut self, amount: f64) -> f64 {
        self.inner.increase_balance(amount as u64) as f64
    }

    #[wasm_bindgen(js_name=reduceBalance)]
    pub fn reduce_balance(&mut self, amount: f64) -> f64 {
        self.inner.reduce_balance(amount as u64) as f64
    }

    #[wasm_bindgen(js_name=setRevision)]
    pub fn set_revision(&mut self, revision: f64) {
        self.inner.set_revision(revision as u64);
    }

    #[wasm_bindgen(js_name=getRevision)]
    pub fn get_revision(&self) -> f64 {
        self.inner.revision() as f64
    }

    #[wasm_bindgen(js_name=setMetadata)]
    pub fn set_metadata(&mut self, metadata: JsValue) -> Result<(), JsValue> {
        if !metadata.is_falsy() {
            let metadata = metadata.to_wasm::<MetadataWasm>("Metadata")?.to_owned();
            self.metadata = Some(metadata.into());
        }

        Ok(())
    }

    #[wasm_bindgen(js_name=getMetadata)]
    pub fn get_metadata(&self) -> Option<MetadataWasm> {
        self.metadata.map(|metadata| metadata.to_owned().into())
    }

    #[wasm_bindgen(js_name=from)]
    pub fn from(object: JsValue) -> Self {
        let i: Identity = serde_json::from_str(&object.as_string().unwrap()).unwrap();
        IdentityWasm {
            inner: i,
            metadata: None,
        }
    }

    #[wasm_bindgen(js_name=toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let mut value = self.inner.to_object().with_js_error()?;

        value
            .replace_at_paths(
                dpp::identity::IDENTIFIER_FIELDS_RAW_OBJECT,
                ReplacementType::TextBase58,
            )
            .map_err(|e| e.to_string())?;

        // Monkey patch public keys data to be deserializable
        let public_keys = value
            .get_array_mut_ref(dpp::identity::property_names::PUBLIC_KEYS)
            .map_err(|e| e.to_string())?;

        for key in public_keys.iter_mut() {
            key.replace_at_paths(
                dpp::identity::identity_public_key::BINARY_DATA_FIELDS,
                ReplacementType::TextBase64,
            )
            .map_err(|e| e.to_string())?;
        }

        let json = value
            .try_into_validating_json()
            .map_err(|e| e.to_string())?
            .to_string();

        js_sys::JSON::parse(&json)
    }

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsValue> {
        let js_public_keys = js_sys::Array::new();
        for pk in self.inner.public_keys().values() {
            let pk_wasm = IdentityPublicKeyWasm::from(pk.to_owned());
            js_public_keys.push(&pk_wasm.to_object()?);
        }

        let value = self.inner.to_object().with_js_error()?;

        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        let js_object = with_js_error!(value.serialize(&serializer))?;

        let id = Buffer::from_bytes(self.inner.id().as_slice());

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
        let bytes =
            PlatformSerializable::serialize_to_bytes(&self.inner.clone()).with_js_error()?;
        Ok(Buffer::from_bytes(&bytes))
    }

    #[wasm_bindgen]
    pub fn hash(&self) -> Result<Vec<u8>, JsValue> {
        self.inner.hash().with_js_error()
    }

    #[wasm_bindgen(js_name=addPublicKey)]
    pub fn add_public_key(&mut self, public_key: IdentityPublicKeyWasm) {
        self.inner
            .public_keys_mut()
            .insert(public_key.get_id(), public_key.into());
    }

    #[wasm_bindgen(js_name=addPublicKeys)]
    pub fn add_public_keys(&mut self, public_keys: js_sys::Array) -> Result<(), JsValue> {
        if public_keys.length() == 0 {
            return Err(format!("Setting public keys failed. The input ('{}') is invalid. You must use array of PublicKeys", public_keys.to_string()).into());
        }

        let public_keys: Vec<IdentityPublicKey> = public_keys
            .iter()
            .map(|key| {
                key.to_wasm::<IdentityPublicKeyWasm>("IdentityPublicKey")
                    .map(|key| key.to_owned().into())
            })
            .collect::<Result<_, _>>()?;

        self.inner.add_public_keys(public_keys);

        Ok(())
    }

    #[wasm_bindgen(js_name=getPublicKeyMaxId)]
    pub fn get_public_key_max_id(&self) -> f64 {
        self.inner.get_public_key_max_id() as f64
    }

    #[wasm_bindgen(js_name=fromBuffer)]
    pub fn from_buffer(buffer: Vec<u8>) -> Result<IdentityWasm, JsValue> {
        let identity: Identity =
            PlatformDeserializable::deserialize_from_bytes(buffer.as_slice()).with_js_error()?;
        Ok(identity.into())
    }
}

impl Inner for IdentityWasm {
    type InnerItem = Identity;

    fn into_inner(self) -> Self::InnerItem {
        self.inner
    }

    fn inner(&self) -> &Self::InnerItem {
        &self.inner
    }

    fn inner_mut(&mut self) -> &mut Self::InnerItem {
        &mut self.inner
    }
}
