//todo: move this file to transition
use dpp::dashcore::anyhow;
use dpp::document::document_transition::document_base_transition::JsonValue;
use dpp::platform_value::BinaryData;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::Convertible;
pub use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};
use wasm_bindgen::prelude::*;

use crate::errors::from_dpp_err;
use crate::utils::WithJsError;
use crate::{buffer::Buffer, utils, with_js_error};

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ToObjectOptions {
    pub skip_signature: Option<bool>,
}

#[wasm_bindgen(js_name=IdentityPublicKeyWithWitness)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IdentityPublicKeyWithWitnessWasm(IdentityPublicKeyInCreation);

#[wasm_bindgen(js_class = IdentityPublicKeyWithWitness)]
impl IdentityPublicKeyWithWitnessWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(raw_public_key: JsValue) -> Result<IdentityPublicKeyWithWitnessWasm, JsValue> {
        let data_string = utils::stringify(&raw_public_key)?;
        let value: JsonValue = serde_json::from_str(&data_string).map_err(|e| e.to_string())?;

        let pk = IdentityPublicKeyInCreation::from_json_object(value).with_js_error()?;

        Ok(IdentityPublicKeyWithWitnessWasm(pk))
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
        self.0.data = BinaryData::new(data);
        Ok(())
    }

    #[wasm_bindgen(js_name=getData)]
    pub fn get_data(&self) -> Buffer {
        Buffer::from_bytes_owned(self.0.data.to_vec())
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

    #[wasm_bindgen(js_name=setSignature)]
    pub fn set_signature(&mut self, signature: Vec<u8>) {
        self.0.signature = BinaryData::new(signature)
    }

    #[wasm_bindgen(js_name=getSignature)]
    pub fn get_signature(&self) -> Vec<u8> {
        self.0.signature.to_vec()
    }

    #[wasm_bindgen(js_name=hash)]
    pub fn hash(&self) -> Result<Vec<u8>, JsValue> {
        self.0.hash_as_vec().with_js_error()
    }

    #[wasm_bindgen(js_name=isMaster)]
    pub fn is_master(&self) -> bool {
        self.0.is_master()
    }

    #[wasm_bindgen(js_name=toJSON)]
    pub fn to_json(&self) -> Result<JsValue, JsValue> {
        let val = self.0.to_json().map_err(from_dpp_err)?;
        let json = val.to_string();
        js_sys::JSON::parse(&json)
    }

    #[wasm_bindgen(js_name=toObject)]
    pub fn to_object(&self, options: JsValue) -> Result<JsValue, JsValue> {
        let opts: ToObjectOptions = if options.is_object() {
            with_js_error!(serde_wasm_bindgen::from_value(options))?
        } else {
            Default::default()
        };

        let val = self
            .0
            .to_raw_json_object(opts.skip_signature.unwrap_or(false))
            .map_err(|e| from_dpp_err(e.into()))?;

        let data_buffer = Buffer::from_bytes(self.0.data.as_slice());

        let json = val.to_string();
        let js_object = js_sys::JSON::parse(&json)?;

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

        if !opts.skip_signature.unwrap_or(false) && !self.0.signature.is_empty() {
            js_sys::Reflect::set(
                &js_object,
                &JsValue::from_str("signature"),
                &JsValue::from(Buffer::from_bytes_owned(self.0.signature.to_vec())),
            )?;
        }

        Ok(js_object)
    }
}

impl IdentityPublicKeyWithWitnessWasm {
    pub fn into_inner(self) -> IdentityPublicKeyInCreation {
        self.0
    }

    pub fn inner(&self) -> &IdentityPublicKeyInCreation {
        &self.0
    }

    pub fn inner_mut(&mut self) -> &mut IdentityPublicKeyInCreation {
        &mut self.0
    }
}

impl From<IdentityPublicKeyInCreation> for IdentityPublicKeyWithWitnessWasm {
    fn from(v: IdentityPublicKeyInCreation) -> Self {
        IdentityPublicKeyWithWitnessWasm(v)
    }
}

impl TryFrom<JsValue> for IdentityPublicKeyWithWitnessWasm {
    type Error = JsValue;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        let str = String::from(js_sys::JSON::stringify(&value)?);
        let val = serde_json::from_str(&str).map_err(|e| from_dpp_err(e.into()))?;
        Ok(Self(
            IdentityPublicKeyInCreation::from_raw_json_object(val).with_js_error()?,
        ))
    }
}

impl From<IdentityPublicKeyWithWitnessWasm> for IdentityPublicKeyInCreation {
    fn from(pk: IdentityPublicKeyWithWitnessWasm) -> Self {
        pk.0
    }
}
