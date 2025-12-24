use crate::utils::error::{format_result_error, ErrorCategory};
use dpp::data_contract::DataContract;
use dpp::document::Document;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::Identity;
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
        id: bs58::encode(identity.id().as_bytes()).into_string(),
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
                data: base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    key.data().as_slice(),
                ),
            })
            .collect(),
    };

    serde_wasm_bindgen::to_value(&identity_json)
        .map_err(|e| format_result_error(ErrorCategory::ConversionError, e))
}

pub fn data_contract_to_js_value(contract: DataContract) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(&contract)
        .map_err(|e| format_result_error(ErrorCategory::ConversionError, e))
}

pub fn document_to_js_value(document: Document) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(&document)
        .map_err(|e| format_result_error(ErrorCategory::ConversionError, e))
}

/// Convert an identifier (32 bytes) to base58 string representation
pub fn identifier_to_base58(id: &[u8; 32]) -> String {
    bs58::encode(id).into_string()
}

/// Convert any byte slice to base58 string representation
pub fn bytes_to_base58(bytes: &[u8]) -> String {
    bs58::encode(bytes).into_string()
}
