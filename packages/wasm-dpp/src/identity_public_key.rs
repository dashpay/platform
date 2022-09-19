use dpp::dashcore::anyhow;
pub use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use wasm_bindgen::prelude::*;

use crate::errors::from_dpp_err;
use crate::identifier::IdentifierWrapper;
use crate::js_buffer::JsBuffer;
use dpp::identity::{IdentityPublicKey, KeyID, KeyType, Purpose, SecurityLevel, TimestampMillis};
use dpp::ProtocolError;
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JsPublicKey {
    pub id: KeyID,
    pub purpose: Purpose,
    pub security_level: SecurityLevel,
    #[serde(rename = "type")]
    pub key_type: KeyType,
    pub data: JsBuffer,
    pub read_only: bool,
    pub disabled_at: Option<TimestampMillis>,
}

impl From<JsPublicKey> for IdentityPublicKey {
    fn from(js_pk: JsPublicKey) -> Self {
        IdentityPublicKey {
            id: js_pk.id,
            purpose: js_pk.purpose,
            security_level: js_pk.security_level,
            key_type: js_pk.key_type,
            data: js_pk.data.data.clone(),
            read_only: js_pk.read_only,
            disabled_at: js_pk.disabled_at,
        }
    }
}

impl From<&JsPublicKey> for IdentityPublicKey {
    fn from(js_pk: &JsPublicKey) -> Self {
        IdentityPublicKey {
            id: js_pk.id,
            purpose: js_pk.purpose,
            security_level: js_pk.security_level,
            key_type: js_pk.key_type,
            data: js_pk.data.data.clone(),
            read_only: js_pk.read_only,
            disabled_at: js_pk.disabled_at,
        }
    }
}

#[wasm_bindgen(js_name=IdentityPublicKey)]
pub struct IdentityPublicKeyWasm(IdentityPublicKey);

// TODO

#[wasm_bindgen(js_class = IdentityPublicKey)]
impl IdentityPublicKeyWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(raw_public_key: JsValue) -> Result<IdentityPublicKeyWasm, JsValue> {
        let streng = String::from(js_sys::JSON::stringify(&raw_public_key)?);
        // let js_public_key: JsPublicKey = serde_wasm_bindgen::from_value(raw_public_key)?;
        let js_public_key: JsPublicKey =
            serde_json::from_str(&streng).map_err(|e| e.to_string())?;
        let pk = IdentityPublicKey::from(js_public_key);
        Ok(pk.into())
    }

    #[wasm_bindgen(js_name=getId)]
    pub fn get_id(&self) -> u32 {
        self.0.get_id() as u32
    }

    #[wasm_bindgen(js_name=setId)]
    pub fn set_id(&mut self, id: u32) {
        self.0.set_id(id.into());
    }

    #[wasm_bindgen(js_name=getType)]
    pub fn get_type(&self) -> u8 {
        self.0.get_type() as u8
    }

    #[wasm_bindgen(js_name=setType)]
    pub fn set_type(&mut self, key_type: u8) -> Result<(), JsValue> {
        self.0.set_type(
            key_type
                .try_into()
                .map_err(|e: anyhow::Error| e.to_string())?,
        );
        Ok(())
    }

    #[wasm_bindgen(js_name=setData)]
    pub fn set_data(&mut self, data: Vec<u8>) -> Result<(), JsValue> {
        self.0.set_data(data);
        Ok(())
    }

    #[wasm_bindgen(js_name=getData)]
    pub fn get_data(&self) -> Vec<u8> {
        self.0.get_data().to_vec()
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
        self.0.get_purpose() as u8
    }

    #[wasm_bindgen(js_name=setSecurityLevel)]
    pub fn set_security_level(&mut self, purpose: u8) -> Result<(), JsValue> {
        self.0.set_security_level(
            purpose
                .try_into()
                .map_err(|e: anyhow::Error| e.to_string())?,
        );
        Ok(())
    }

    #[wasm_bindgen(js_name=getSecurityLevel)]
    pub fn get_security_level(&self) -> u8 {
        self.0.get_security_level() as u8
    }

    #[wasm_bindgen(js_name=setReadOnly)]
    pub fn set_readonly(&mut self, purpose: bool) {
        self.0.set_readonly(purpose);
    }

    #[wasm_bindgen(js_name=isReadOnly)]
    pub fn is_readonly(&self) -> bool {
        self.0.get_readonly()
    }

    #[wasm_bindgen(js_name=setDisabledAt)]
    pub fn set_disabled_at(&mut self, timestamp: u32) {
        self.0.set_disabled_at(timestamp as u64);
    }

    #[wasm_bindgen(js_name=getDisabledAt)]
    pub fn get_disabled_at(&self) -> Option<u32> {
        self.0.get_disabled_at().map(|timestamp| timestamp as u32)
    }

    #[wasm_bindgen(js_name=hash)]
    pub fn hash(&self) -> Result<Vec<u8>, JsValue> {
        self.0.hash().map_err(|e| from_dpp_err(e))
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
}

impl From<IdentityPublicKey> for IdentityPublicKeyWasm {
    fn from(v: IdentityPublicKey) -> Self {
        IdentityPublicKeyWasm(v)
    }
}
