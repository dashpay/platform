//! Response types for DAPI client

use serde::{Deserialize, Serialize};
use super::types::*;

/// Broadcast response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastResponse {
    #[serde(rename = "transactionId")]
    pub transaction_id: String,
    #[serde(rename = "blockHeight", skip_serializing_if = "Option::is_none")]
    pub block_height: Option<u64>,
    #[serde(rename = "blockHash", skip_serializing_if = "Option::is_none")]
    pub block_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<BroadcastError>,
}

/// Broadcast error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastError {
    pub code: u32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Get identity response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetIdentityResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<Identity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof: Option<String>, // Base64 encoded proof
    pub metadata: ResponseMetadata,
}

/// Get data contract response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetDataContractResponse {
    #[serde(rename = "dataContract", skip_serializing_if = "Option::is_none")]
    pub data_contract: Option<DataContract>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof: Option<String>, // Base64 encoded proof
    pub metadata: ResponseMetadata,
}

/// Get documents response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetDocumentsResponse {
    pub documents: Vec<Document>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof: Option<String>, // Base64 encoded proof
    pub metadata: ResponseMetadata,
}

/// Get epoch info response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetEpochInfoResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epoch: Option<EpochInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof: Option<String>, // Base64 encoded proof
    pub metadata: ResponseMetadata,
}

/// Wait for state transition response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitForStateTransitionResponse {
    pub result: StateTransitionResult,
}

/// Get identity balance response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetIdentityBalanceResponse {
    pub balance: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof: Option<String>, // Base64 encoded proof
    pub metadata: ResponseMetadata,
}

/// Get identity nonce response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetIdentityNonceResponse {
    pub nonce: u64,
}

/// Protocol version response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetProtocolVersionResponse {
    pub version: ProtocolVersionInfo,
}