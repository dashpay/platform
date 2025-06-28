//! WASM-compatible type definitions that mirror platform_proto types
//!
//! These types provide a lightweight alternative to protobuf definitions
//! and are designed to work seamlessly in WASM environments.

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Identity representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub id: String,
    pub balance: u64,
    pub revision: u64,
    pub public_keys: Vec<IdentityPublicKey>,
}

/// Identity public key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityPublicKey {
    pub id: u32,
    pub purpose: u32,
    pub security_level: u32,
    pub key_type: u32,
    pub data: Vec<u8>,
}

/// Data contract representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataContract {
    pub id: String,
    pub owner_id: String,
    pub schema: serde_json::Value,
    pub version: u32,
}

/// Document representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub contract_id: String,
    pub document_type: String,
    pub owner_id: String,
    pub revision: u64,
    pub data: serde_json::Value,
}

/// State transition result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransitionResult {
    pub block_height: u64,
    pub block_hash: String,
    pub transaction_hash: String,
    pub status: StateTransitionStatus,
    pub error: Option<String>,
}

/// State transition status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StateTransitionStatus {
    Success,
    Failed,
    Pending,
}

/// Proof response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofResponse<T> {
    pub data: Option<T>,
    pub proof: Option<Vec<u8>>,
    pub metadata: ResponseMetadata,
}

/// Response metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
    pub height: u64,
    pub core_chain_locked_height: u32,
    pub time_ms: u64,
    pub protocol_version: u32,
}

/// Epoch info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpochInfo {
    pub number: u32,
    pub first_block_height: u64,
    pub first_core_block_height: u32,
    pub start_time: u64,
    pub fee_multiplier: f64,
}

/// Protocol version info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolVersionInfo {
    pub version: u32,
    pub min_supported_version: u32,
    pub latest_version: u32,
}

/// Convert types to/from JavaScript

impl Identity {
    /// Convert to JavaScript object
    pub fn to_js_object(&self) -> Result<JsValue, JsError> {
        serde_wasm_bindgen::to_value(self)
            .map_err(|e| JsError::new(&format!("Failed to convert Identity: {}", e)))
    }

    /// Convert from JavaScript object
    pub fn from_js_object(obj: JsValue) -> Result<Self, JsError> {
        serde_wasm_bindgen::from_value(obj)
            .map_err(|e| JsError::new(&format!("Failed to parse Identity: {}", e)))
    }
}

impl DataContract {
    /// Convert to JavaScript object
    pub fn to_js_object(&self) -> Result<JsValue, JsError> {
        serde_wasm_bindgen::to_value(self)
            .map_err(|e| JsError::new(&format!("Failed to convert DataContract: {}", e)))
    }

    /// Convert from JavaScript object
    pub fn from_js_object(obj: JsValue) -> Result<Self, JsError> {
        serde_wasm_bindgen::from_value(obj)
            .map_err(|e| JsError::new(&format!("Failed to parse DataContract: {}", e)))
    }
}

impl Document {
    /// Convert to JavaScript object
    pub fn to_js_object(&self) -> Result<JsValue, JsError> {
        serde_wasm_bindgen::to_value(self)
            .map_err(|e| JsError::new(&format!("Failed to convert Document: {}", e)))
    }

    /// Convert from JavaScript object
    pub fn from_js_object(obj: JsValue) -> Result<Self, JsError> {
        serde_wasm_bindgen::from_value(obj)
            .map_err(|e| JsError::new(&format!("Failed to parse Document: {}", e)))
    }
}
