use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
use dpp::identity::{Identity, IdentityPublicKey, IdentityV0, KeyID};
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::metadata::Metadata;
use dpp::serialization::ValueConvertible;
use dpp::version::PlatformVersion;
use crate::identifier::IdentifierWrapper;
use crate::{utils, with_js_error};
use crate::errors::from_dpp_err;
use crate::utils::{IntoWasm, WithJsError};
use dpp::platform_value::{ReplacementType, Value};
use crate::identity::IdentityPublicKeyWasm;
use crate::metadata::MetadataWasm;

#[wasm_bindgen(js_name=Identity)]
#[derive(Clone)]
pub struct IdentityWasm {
    inner: Identity,
    metadata: Option<Metadata>
}

// impl From<IdentityWasm> for Identity {
//     fn from(identity: IdentityWasm) -> Self {
//         identity.0
//     }
// }
//
// impl From<Identity> for IdentityWasm {
//     fn from(identity: Identity) -> Self {
//         Self(identity)
//     }
// }
#[wasm_bindgen(js_class=Identity)]
impl IdentityWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(raw_identity: JsValue) -> Result<IdentityWasm, JsValue> {
        let identity_json_string = utils::stringify(&raw_identity)?;
        let identity_json: JsonValue = serde_json::from_str(&identity_json_string)
            .map_err(|e| e.to_string())?;

        // Monkey patch identifier to be deserializable
        let mut identity_platform_value: Value = identity_json.into();
        identity_platform_value.replace_at_paths(
            dpp::identity::IDENTIFIER_FIELDS_RAW_OBJECT,
            ReplacementType::TextBase64
        ).map_err(|e| e.to_string())?;

        // Monkey patch public keys data to be deserializable
        let mut public_keys = identity_platform_value.get_array_mut_ref(dpp::identity::property_names::PUBLIC_KEYS)
            .map_err(|e| e.to_string())?;

        for key in public_keys.iter_mut() {
            key.replace_at_paths(
                dpp::identity::identity_public_key::BINARY_DATA_FIELDS,
                ReplacementType::TextBase64
            ).map_err(|e| e.to_string())?;
        }


        let identity: Identity = Identity::from_object(identity_platform_value)
            .map_err(from_dpp_err)?;

        Ok(IdentityWasm {
            inner: identity,
            metadata: None
        })
    }

    #[wasm_bindgen(js_name=getId)]
    pub fn get_id(&self) -> IdentifierWrapper {
        self.inner.id().into()
    }

    #[wasm_bindgen(js_name=setPublicKeys)]
    pub fn set_public_keys(&mut self, public_keys: js_sys::Array) -> Result<usize, JsValue> {
        if public_keys.length() == 0 {
            return Err(format!("Setting public keys failed. The input ('{}') is invalid. You must use array of PublicKeys", public_keys.to_string()).into());
        }

        let public_keys = public_keys
            .into_iter()
            .map(|key| IdentityPublicKeyWasm::new(key).map(|key| {
                let key = IdentityPublicKey::from(key);
                (key.id(), key)
            }))
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
    //
    // #[wasm_bindgen(js_name=setAssetLockProof)]
    // pub fn set_asset_lock_proof(&mut self, lock: JsValue) -> Result<(), JsValue> {
    //     let asset_lock_proof = create_asset_lock_proof_from_wasm_instance(&lock)?;
    //     self.inner.set_asset_lock_proof(asset_lock_proof);
    //     Ok(())
    // }
    //
    // #[wasm_bindgen(js_name=getAssetLockProof)]
    // pub fn get_asset_lock_proof(&self) -> Option<AssetLockProofWasm> {
    //     self.inner
    //         .get_asset_lock_proof()
    //         .map(AssetLockProof::to_owned)
    //         .map(Into::into)
    // }
    //
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
        self.metadata
            .map(|metadata| metadata.to_owned().into())
    }

    #[wasm_bindgen(js_name=from)]
    pub fn from(object: JsValue) -> Self {
        let i: Identity = serde_json::from_str(&object.as_string().unwrap()).unwrap();
        IdentityWasm {
            inner: i,
            metadata: None
        }
    }
    //
    // #[wasm_bindgen(js_name=toJSON)]
    // pub fn to_json(&self) -> Result<JsValue, JsValue> {
    //     let json = self.inner.to_json().with_js_error()?;
    //     let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    //     with_js_error!(json.serialize(&serializer))
    // }
    //
    // #[wasm_bindgen(js_name=toObject)]
    // pub fn to_object(&self) -> Result<JsValue, JsValue> {
    //     let js_public_keys = js_sys::Array::new();
    //     for pk in self.inner.public_keys().values() {
    //         let pk_wasm = IdentityPublicKeyWasm::from(pk.to_owned());
    //         js_public_keys.push(&pk_wasm.to_object()?);
    //     }
    //
    //     let value = self.inner.to_cleaned_object().with_js_error()?;
    //
    //     let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    //     let js_object = with_js_error!(value.serialize(&serializer))?;
    //
    //     let id = Buffer::from_bytes(self.inner.id.as_slice());
    //
    //     js_sys::Reflect::set(&js_object, &"id".to_owned().into(), &id)?;
    //
    //     js_sys::Reflect::set(
    //         &js_object,
    //         &"publicKeys".to_owned().into(),
    //         &JsValue::from(&js_public_keys),
    //     )?;
    //
    //     Ok(js_object)
    // }
    //
    // #[wasm_bindgen(js_name=toBuffer)]
    // pub fn to_buffer(&self) -> Result<Buffer, JsValue> {
    //     let bytes = PlatformSerializable::serialize(&self.inner.clone()).with_js_error()?;
    //     Ok(Buffer::from_bytes(&bytes))
    // }
    //
    // #[wasm_bindgen]
    // pub fn hash(&self) -> Result<Vec<u8>, JsValue> {
    //     self.inner.hash().with_js_error()
    // }
    //
    #[wasm_bindgen(js_name=addPublicKey)]
    pub fn add_public_key(&mut self, public_key: IdentityPublicKeyWasm) {
        self.inner
            .public_keys_mut()
            .insert(public_key.get_id(), public_key.into());
    }

    // The method `addPublicKeys()` takes an variadic array of `IdentityPublicKeyWasm` as an input. But elements of the array
    // are available ONLY as `JsValue`. WASM-bindgen uses output from `toJSON()` to store WASM-object as `JsValue`.
    // `toJSON()` converts binary data to `base64` or `base58`. Therefore we need to use `from_json_object()` constructor to
    // to convert strings back into bytes and get `IdentityPublicKeyWasm`
    #[wasm_bindgen(js_name=addPublicKeys, variadic)]
    pub fn add_public_keys(&mut self, public_keys: js_sys::Array) -> Result<(), JsValue> {
        if public_keys.length() == 0 {
            return Err(format!("Setting public keys failed. The input ('{}') is invalid. You must use array of PublicKeys", public_keys.to_string()).into());
        }

        let public_keys: Vec<IdentityPublicKey> = public_keys
            .into_iter()
            .map(|key| key
                .to_wasm::<IdentityPublicKeyWasm>("IdentityPublicKey")
                .map(|key| key.to_owned().into())
            )
            .collect::<Result<_, _>>()?;

        self.inner.add_public_keys(public_keys);

        Ok(())
    }

    #[wasm_bindgen(js_name=getPublicKeyMaxId)]
    pub fn get_public_key_max_id(&self) -> f64 {
        self.inner.get_public_key_max_id() as f64
    }
    //
    // #[wasm_bindgen(js_name=fromBuffer)]
    // pub fn from_buffer(buffer: Vec<u8>) -> Result<IdentityWasm, JsValue> {
    //     let identity: Identity =
    //         PlatformDeserializable::deserialize(buffer.as_slice()).with_js_error()?;
    //     Ok(identity.into())
    // }
}
