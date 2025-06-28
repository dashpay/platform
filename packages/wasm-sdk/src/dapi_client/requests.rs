//! Request types for DAPI client

use serde::{Deserialize, Serialize};

/// Broadcast state transition request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastRequest {
    #[serde(rename = "stateTransition", with = "base64")]
    pub state_transition: Vec<u8>,
    pub wait: bool,
}

/// Get identity request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetIdentityRequest {
    #[serde(rename = "identityId")]
    pub identity_id: String,
    pub prove: bool,
}

/// Get data contract request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetDataContractRequest {
    #[serde(rename = "contractId")]
    pub contract_id: String,
    pub prove: bool,
}

/// Get documents request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetDocumentsRequest {
    #[serde(rename = "contractId")]
    pub contract_id: String,
    #[serde(rename = "documentType")]
    pub document_type: String,
    #[serde(rename = "where")]
    pub where_clause: serde_json::Value,
    #[serde(rename = "orderBy")]
    pub order_by: serde_json::Value,
    pub limit: u32,
    #[serde(rename = "startAfter", skip_serializing_if = "Option::is_none")]
    pub start_after: Option<String>,
    pub prove: bool,
}

/// Get epoch info request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetEpochInfoRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epoch: Option<u32>,
    pub prove: bool,
}

/// Wait for state transition request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitForStateTransitionRequest {
    #[serde(rename = "stateTransitionHash")]
    pub state_transition_hash: String,
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: u32,
}

/// Get identity balance request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetIdentityBalanceRequest {
    #[serde(rename = "identityId")]
    pub identity_id: String,
    pub prove: bool,
}

/// Get identity nonce request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetIdentityNonceRequest {
    #[serde(rename = "identityId")]
    pub identity_id: String,
    #[serde(rename = "contractId")]
    pub contract_id: String,
}

/// Subscribe to state transitions request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeToStateTransitionsRequest {
    pub query: StateTransitionQuery,
}

/// State transition query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransitionQuery {
    #[serde(
        rename = "stateTransitionTypes",
        skip_serializing_if = "Option::is_none"
    )]
    pub state_transition_types: Option<Vec<String>>,
    #[serde(rename = "identityIds", skip_serializing_if = "Option::is_none")]
    pub identity_ids: Option<Vec<String>>,
    #[serde(rename = "contractIds", skip_serializing_if = "Option::is_none")]
    pub contract_ids: Option<Vec<String>>,
}

/// Custom base64 serialization for binary data
mod base64 {
    use base64::Engine;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
        serializer.serialize_str(&encoded)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let encoded = String::deserialize(deserializer)?;
        base64::engine::general_purpose::STANDARD
            .decode(&encoded)
            .map_err(serde::de::Error::custom)
    }
}
