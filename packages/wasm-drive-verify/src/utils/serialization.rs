use dpp::identity::Identity;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::data_contract::DataContract;
use dpp::document::Document;
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[derive(Serialize)]
pub struct IdentityJson {
    pub id: String,
    pub balance: u64,
    pub revision: u64,
    pub public_keys: Vec<PublicKeyJson>,
}

#[derive(Serialize)]
pub struct PublicKeyJson {
    pub id: u32,
    pub purpose: u8,
    pub security_level: u8,
    pub key_type: u8,
    pub data: String,
}

pub fn identity_to_js_value(identity: Identity) -> Result<JsValue, JsValue> {
    let identity_json = IdentityJson {
        id: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, identity.id().as_bytes()),
        balance: identity.balance(),
        revision: identity.revision(),
        public_keys: identity
            .public_keys()
            .values()
            .map(|key| PublicKeyJson {
                id: key.id(),
                purpose: key.purpose() as u8,
                security_level: key.security_level() as u8,
                key_type: key.key_type() as u8,
                data: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, key.data().as_slice()),
            })
            .collect(),
    };
    
    serde_wasm_bindgen::to_value(&identity_json)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize identity: {}", e)))
}

pub fn data_contract_to_js_value(contract: DataContract) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(&contract)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize contract: {}", e)))
}

pub fn document_to_js_value(document: Document) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(&document)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize document: {}", e)))
}