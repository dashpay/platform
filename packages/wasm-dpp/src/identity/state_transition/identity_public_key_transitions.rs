//todo: move this file to transition
use dpp::platform_value::{BinaryData, ReplacementType};
use dpp::serialization::ValueConvertible;
use dpp::state_transition::public_key_in_creation::accessors::{
    IdentityPublicKeyInCreationV0Getters, IdentityPublicKeyInCreationV0Setters,
};

use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use js_sys::Reflect::delete_property;
pub use serde::{Deserialize, Serialize};

use dpp::identity::contract_bounds::ContractBounds;
use dpp::ProtocolError;
use std::convert::TryInto;
use wasm_bindgen::prelude::*;

use crate::errors::from_dpp_err;
use crate::identifier::IdentifierWrapper;
use crate::utils::WithJsError;
use crate::{buffer::Buffer, with_js_error};
use dpp::version::PlatformVersion;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ToObjectOptions {
    #[allow(dead_code)]
    #[deprecated(note = "This function is marked as unused.")]
    pub skip_signature: Option<bool>,
}

#[wasm_bindgen(js_name=IdentityPublicKeyWithWitness)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IdentityPublicKeyWithWitnessWasm(IdentityPublicKeyInCreation);

#[wasm_bindgen(js_class = IdentityPublicKeyWithWitness)]
impl IdentityPublicKeyWithWitnessWasm {
    #[wasm_bindgen(constructor)]
    pub fn new(platform_version: u32) -> Result<IdentityPublicKeyWithWitnessWasm, JsValue> {
        let platform_version =
            &PlatformVersion::get(platform_version).map_err(|e| JsValue::from(e.to_string()))?;

        IdentityPublicKeyInCreation::default_versioned(platform_version)
            .map(Into::into)
            .map_err(from_dpp_err)
    }

    #[wasm_bindgen(js_name=getId)]
    pub fn get_id(&self) -> u32 {
        self.0.id()
    }

    #[wasm_bindgen(js_name=setId)]
    pub fn set_id(&mut self, id: u32) {
        self.0.set_id(id);
    }

    #[wasm_bindgen(js_name=getType)]
    pub fn get_type(&self) -> u8 {
        self.0.key_type() as u8
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

    #[wasm_bindgen(js_name=setContractBounds)]
    pub fn set_contract_bounds(
        &mut self,
        contract_id: IdentifierWrapper,
        document_type_name: Option<String>,
    ) {
        let contract_bounds = if document_type_name.is_some() {
            ContractBounds::SingleContractDocumentType {
                id: contract_id.into(),
                document_type_name: document_type_name.unwrap(),
            }
        } else {
            ContractBounds::SingleContract {
                id: contract_id.into(),
            }
        };
        self.0.set_contract_bounds(Some(contract_bounds))
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

    #[wasm_bindgen(js_name=setSignature)]
    pub fn set_signature(&mut self, signature: Vec<u8>) {
        self.0.set_signature(BinaryData::new(signature))
    }

    #[wasm_bindgen(js_name=getSignature)]
    pub fn get_signature(&self) -> Vec<u8> {
        self.0.signature().to_vec()
    }

    #[wasm_bindgen(js_name=hash)]
    pub fn hash(&self) -> Result<Vec<u8>, JsValue> {
        self.0.hash_as_vec().with_js_error()
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
    pub fn to_object(&self, skip_signature: bool) -> Result<JsValue, JsValue> {
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

        if !skip_signature {
            let signature_buffer = Buffer::from_bytes(self.0.signature().as_slice());
            js_sys::Reflect::set(
                &js_object,
                &JsValue::from_str("signature"),
                &JsValue::from(signature_buffer),
            )?;
        } else {
            let js_object = js_sys::Object::from(js_object);
            delete_property(&js_object, &JsValue::from_str("signature"))?;
            return Ok(js_object.into());
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

// TODO(versioning): restore
// impl TryFrom<JsValue> for IdentityPublicKeyWithWitnessWasm {
//     type Error = JsValue;
//
//     fn try_from(value: JsValue) -> Result<Self, Self::Error> {
//         let str = String::from(js_sys::JSON::stringify(&value)?);
//         let val = serde_json::from_str(&str).map_err(|e| from_dpp_err(e.into()))?;
//         Ok(Self(
//             IdentityPublicKeyInCreation::from_raw_json_object(val).with_js_error()?,
//         ))
//     }
// }

impl From<IdentityPublicKeyWithWitnessWasm> for IdentityPublicKeyInCreation {
    fn from(pk: IdentityPublicKeyWithWitnessWasm) -> Self {
        pk.0
    }
}
