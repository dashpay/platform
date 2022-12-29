use dpp::dashcore::anyhow;
pub use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};
use wasm_bindgen::prelude::*;

use crate::errors::from_dpp_err;
use crate::{buffer::Buffer, utils};
use dpp::identity::IdentityPublicKey;

mod purpose;
pub use purpose::*;

mod security_level;
pub use security_level::*;

mod key_type;

pub use key_type::*;

#[wasm_bindgen(js_name=IdentityPublicKey)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IdentityPublicKeyWasm(IdentityPublicKey);

#[wasm_bindgen(js_class = IdentityPublicKey)]
impl IdentityPublicKeyWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(raw_public_key: JsValue) -> Result<IdentityPublicKeyWasm, JsValue> {
        let data_string = utils::stringify(&raw_public_key)?;
        let pk: IdentityPublicKeyWasm =
            serde_json::from_str(&data_string).map_err(|e| e.to_string())?;

        Ok(pk)
    }

    #[wasm_bindgen(js_name=getId)]
    pub fn get_id(&self) -> u32 {
        self.0.id
    }

    #[wasm_bindgen(js_name=setId)]
    pub fn set_id(&mut self, id: u32) {
        self.0.id = id;
    }

    #[wasm_bindgen(js_name=getType)]
    pub fn get_type(&self) -> u8 {
        self.0.key_type as u8
    }

    #[wasm_bindgen(js_name=setType)]
    pub fn set_type(&mut self, key_type: u8) -> Result<(), JsValue> {
        self.0.key_type = key_type
            .try_into()
            .map_err(|e: anyhow::Error| e.to_string())?;
        Ok(())
    }

    #[wasm_bindgen(js_name=setData)]
    pub fn set_data(&mut self, data: Vec<u8>) -> Result<(), JsValue> {
        self.0.set_data(data);
        Ok(())
    }

    #[wasm_bindgen(js_name=getData)]
    pub fn get_data(&self) -> Vec<u8> {
        self.0.data.clone()
    }

    #[wasm_bindgen(js_name=setPurpose)]
    pub fn set_purpose(&mut self, purpose: u8) -> Result<(), JsValue> {
        self.0.purpose = purpose
            .try_into()
            .map_err(|e: anyhow::Error| e.to_string())?;
        Ok(())
    }

    #[wasm_bindgen(js_name=getPurpose)]
    pub fn get_purpose(&self) -> u8 {
        self.0.purpose as u8
    }

    #[wasm_bindgen(js_name=setSecurityLevel)]
    pub fn set_security_level(&mut self, security_level: u8) -> Result<(), JsValue> {
        self.0.security_level = security_level
            .try_into()
            .map_err(|e: anyhow::Error| e.to_string())?;
        Ok(())
    }

    #[wasm_bindgen(js_name=getSecurityLevel)]
    pub fn get_security_level(&self) -> u8 {
        self.0.security_level as u8
    }

    #[wasm_bindgen(js_name=setReadOnly)]
    pub fn set_read_only(&mut self, read_only: bool) {
        self.0.read_only = read_only;
    }

    #[wasm_bindgen(js_name=isReadOnly)]
    pub fn is_read_only(&self) -> bool {
        self.0.read_only
    }

    #[wasm_bindgen(js_name=setDisabledAt)]
    pub fn set_disabled_at(&mut self, timestamp: u32) {
        self.0.set_disabled_at(timestamp as u64);
    }

    #[wasm_bindgen(js_name=getDisabledAt)]
    pub fn get_disabled_at(&self) -> Option<f64> {
        self.0.disabled_at.map(|timestamp| timestamp as f64)
    }

    #[wasm_bindgen(js_name=hash)]
    pub fn hash(&self) -> Result<Vec<u8>, JsValue> {
        self.0.hash().map_err(from_dpp_err)
    }

    #[wasm_bindgen(js_name=isMaster)]
    pub fn is_master(&self) -> bool {
        self.0.is_master()
    }

    #[wasm_bindgen(js_name=toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let val = self.0.to_json().map_err(|e| from_dpp_err(e.into()))?;
        let json = val.to_string();
        js_sys::JSON::parse(&json)
    }

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(&self) -> Result<JsValue, JsValue> {
        let val = self
            .0
            .to_raw_json_object()
            .map_err(|e| from_dpp_err(e.into()))?;

        let data_buffer = Buffer::from_bytes(self.0.get_data());

        let json = val.to_string();
        let js_object = js_sys::JSON::parse(&json)?;

        js_sys::Reflect::set(
            &js_object,
            &"data".to_owned().into(),
            &JsValue::from(data_buffer),
        )?;

        Ok(js_object)
    }
}

impl From<IdentityPublicKey> for IdentityPublicKeyWasm {
    fn from(v: IdentityPublicKey) -> Self {
        IdentityPublicKeyWasm(v)
    }
}

impl TryFrom<JsValue> for IdentityPublicKeyWasm {
    type Error = JsValue;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        let str = String::from(js_sys::JSON::stringify(&value)?);
        let val = serde_json::from_str(&str).map_err(|e| from_dpp_err(e.into()))?;
        Ok(Self(
            IdentityPublicKey::from_raw_object(val).map_err(from_dpp_err)?,
        ))
    }
}

impl From<IdentityPublicKeyWasm> for IdentityPublicKey {
    fn from(pk: IdentityPublicKeyWasm) -> Self {
        pk.0
    }
}
