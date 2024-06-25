pub use serde::Serialize;

use std::convert::TryInto;
use wasm_bindgen::prelude::*;

use crate::utils::WithJsError;
use crate::{buffer::Buffer, with_js_error};
use dpp::identity::identity_public_key::accessors::v0::{
    IdentityPublicKeyGettersV0, IdentityPublicKeySettersV0,
};
use dpp::identity::identity_public_key::hash::IdentityPublicKeyHashMethodsV0;
use dpp::identity::{IdentityPublicKey, KeyID, TimestampMillis};
use dpp::platform_value::{BinaryData, ReplacementType};
use dpp::serialization::{PlatformDeserializable, PlatformSerializable, ValueConvertible};
use dpp::ProtocolError;

use dpp::version::PlatformVersion;
mod purpose;
pub use purpose::*;

mod security_level;
pub use security_level::*;

mod key_type;

use crate::errors::from_dpp_err;
pub use key_type::*;

#[wasm_bindgen(js_name=IdentityPublicKey)]
#[derive(Debug, Clone)]
pub struct IdentityPublicKeyWasm(IdentityPublicKey);

#[wasm_bindgen(js_class = IdentityPublicKey)]
impl IdentityPublicKeyWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(platform_version: u32) -> Result<IdentityPublicKeyWasm, JsValue> {
        let platform_version =
            &PlatformVersion::get(platform_version).map_err(|e| JsValue::from(e.to_string()))?;

        IdentityPublicKey::default_versioned(platform_version)
            .map(Into::into)
            .map_err(from_dpp_err)
    }

    #[wasm_bindgen(js_name=getId)]
    pub fn get_id(&self) -> KeyID {
        self.0.id()
    }

    #[wasm_bindgen(js_name=setId)]
    pub fn set_id(&mut self, id: KeyID) {
        self.0.set_id(id);
    }

    #[wasm_bindgen(js_name=getType)]
    pub fn get_type(&self) -> u8 {
        self.0.key_type() as u8
    }

    #[wasm_bindgen(js_name=setType)]
    pub fn set_type(&mut self, key_type: u8) -> Result<(), JsValue> {
        self.0.set_key_type(
            key_type
                .try_into()
                .map_err(|e: anyhow::Error| e.to_string())?,
        );
        Ok(())
    }

    #[wasm_bindgen(js_name=setData)]
    pub fn set_data(&mut self, data: Vec<u8>) -> Result<(), JsValue> {
        self.0.set_data(BinaryData::new(data));
        Ok(())
    }

    #[wasm_bindgen(js_name=getData)]
    pub fn get_data(&self) -> Buffer {
        Buffer::from_bytes_owned(self.0.data().to_vec())
    }

    #[wasm_bindgen(js_name=setPurpose)]
    pub fn set_purpose(&mut self, purpose: u8) -> Result<(), JsValue> {
        self.0.set_purpose(
            purpose
                .try_into()
                .map_err(|e: anyhow::Error| e.to_string())?,
        );
        Ok(())
    }

    #[wasm_bindgen(js_name=getPurpose)]
    pub fn get_purpose(&self) -> u8 {
        self.0.purpose() as u8
    }

    #[wasm_bindgen(js_name=setSecurityLevel)]
    pub fn set_security_level(&mut self, security_level: u8) -> Result<(), JsValue> {
        self.0.set_security_level(
            security_level
                .try_into()
                .map_err(|e: ProtocolError| e.to_string())?,
        );
        Ok(())
    }

    #[wasm_bindgen(js_name=getSecurityLevel)]
    pub fn get_security_level(&self) -> u8 {
        self.0.security_level() as u8
    }

    #[wasm_bindgen(js_name=setReadOnly)]
    pub fn set_read_only(&mut self, read_only: bool) {
        self.0.set_read_only(read_only);
    }

    #[wasm_bindgen(js_name=isReadOnly)]
    pub fn is_read_only(&self) -> bool {
        self.0.read_only()
    }

    #[wasm_bindgen(js_name=setDisabledAt)]
    pub fn set_disabled_at(&mut self, timestamp: js_sys::Date) {
        self.0
            .set_disabled_at(timestamp.get_time() as TimestampMillis);
    }

    #[wasm_bindgen(js_name=getDisabledAt)]
    pub fn get_disabled_at(&self) -> Option<js_sys::Date> {
        self.0
            .disabled_at()
            .map(|timestamp| js_sys::Date::new(&JsValue::from_f64(timestamp as f64)))
    }

    #[wasm_bindgen(js_name=hash)]
    pub fn hash(&self) -> Result<Vec<u8>, JsValue> {
        self.0
            .public_key_hash()
            .map(|result| result.to_vec())
            .with_js_error()
    }

    #[wasm_bindgen(js_name=isMaster)]
    pub fn is_master(&self) -> bool {
        self.0.is_master()
    }

    #[wasm_bindgen(js_name=toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let mut value = self.0.to_object().map_err(from_dpp_err)?;

        value
            .replace_at_paths(
                dpp::identity::identity_public_key::BINARY_DATA_FIELDS,
                ReplacementType::TextBase64,
            )
            .map_err(|e| e.to_string())?;

        let json = value
            .try_into_validating_json()
            .map_err(|e| e.to_string())?
            .to_string();

        js_sys::JSON::parse(&json)
    }

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsValue> {
        let value = self.0.to_object().map_err(from_dpp_err)?;

        let data_buffer = Buffer::from_bytes(self.0.data().as_slice());

        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        let js_object = with_js_error!(value.serialize(&serializer))?;

        js_sys::Reflect::set(
            &js_object,
            &JsValue::from_str("type"),
            &JsValue::from(self.get_type()),
        )?;

        js_sys::Reflect::set(
            &js_object,
            &JsValue::from_str("data"),
            &JsValue::from(data_buffer),
        )?;

        Ok(js_object)
    }

    #[wasm_bindgen(js_name=toBuffer)]
    pub fn to_buffer(&self) -> Result<Buffer, JsValue> {
        let bytes = PlatformSerializable::serialize_to_bytes(&self.0.clone()).with_js_error()?;
        Ok(Buffer::from_bytes(&bytes))
    }

    #[wasm_bindgen(js_name=fromBuffer)]
    pub fn from_buffer(buffer: Vec<u8>) -> Result<IdentityPublicKeyWasm, JsValue> {
        let key: IdentityPublicKey =
            PlatformDeserializable::deserialize_from_bytes(buffer.as_slice()).with_js_error()?;
        Ok(key.into())
    }
}

// impl Inner for IdentityPublicKeyWasm {
//     type InnerItem = IdentityPublicKey;
//
//     fn into_inner(self) -> IdentityPublicKey {
//         self.0
//     }
//
//     fn inner(&self) -> &IdentityPublicKey {
//         &self.0
//     }
//
//     fn inner_mut(&mut self) -> &mut IdentityPublicKey {
//         &mut self.0
//     }
// }

impl From<IdentityPublicKey> for IdentityPublicKeyWasm {
    fn from(v: IdentityPublicKey) -> Self {
        IdentityPublicKeyWasm(v)
    }
}
//
// impl TryFrom<JsValue> for IdentityPublicKeyWasm {
//     type Error = JsValue;
//
//     fn try_from(value: JsValue) -> Result<Self, Self::Error> {
//         let str = String::from(js_sys::JSON::stringify(&value)?);
//         let val = serde_json::from_str(&str).map_err(|e| from_dpp_err(e.into()))?;
//         Ok(Self(IdentityPublicKey::from_value(val).with_js_error()?))
//     }
// }
//
impl From<IdentityPublicKeyWasm> for IdentityPublicKey {
    fn from(pk: IdentityPublicKeyWasm) -> Self {
        pk.0
    }
}
