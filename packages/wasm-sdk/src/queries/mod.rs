pub mod identity;
pub mod data_contract;
pub mod document;
pub mod dpns;
pub mod protocol;
pub mod epoch;
pub mod token;
pub mod voting;
pub mod group;
pub mod system;

// Re-export all query functions for easy access
pub use identity::*;
pub use data_contract::*;
pub use document::*;
pub use dpns::*;
pub use protocol::*;
pub use epoch::*;
pub use token::*;
pub use voting::*;
pub use group::*;
pub use system::*;

use serde::{Serialize, Deserialize};

// Common response structure for queries with proof and metadata
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProofMetadataResponse<T> {
    pub data: T,
    pub metadata: ResponseMetadata,
    pub proof: ProofInfo,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ResponseMetadata {
    pub height: u64,
    pub core_chain_locked_height: u32,
    pub epoch: u32,
    pub time_ms: u64,
    pub protocol_version: u32,
    pub chain_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProofInfo {
    pub grovedb_proof: String, // Base64 encoded
    pub quorum_hash: String,   // Hex encoded
    pub signature: String,     // Base64 encoded
    pub round: u32,
    pub block_id_hash: String, // Hex encoded
    pub quorum_type: u32,
}

// Helper function to convert platform ResponseMetadata to our ResponseMetadata
impl From<dash_sdk::platform::proto::ResponseMetadata> for ResponseMetadata {
    fn from(metadata: dash_sdk::platform::proto::ResponseMetadata) -> Self {
        ResponseMetadata {
            height: metadata.height,
            core_chain_locked_height: metadata.core_chain_locked_height,
            epoch: metadata.epoch,
            time_ms: metadata.time_ms,
            protocol_version: metadata.protocol_version,
            chain_id: metadata.chain_id,
        }
    }
}

// Helper function to convert platform Proof to our ProofInfo
impl From<dash_sdk::platform::proto::Proof> for ProofInfo {
    fn from(proof: dash_sdk::platform::proto::Proof) -> Self {
        use base64::{Engine as _, engine::general_purpose};
        
        ProofInfo {
            grovedb_proof: general_purpose::STANDARD.encode(&proof.grovedb_proof),
            quorum_hash: hex::encode(&proof.quorum_hash),
            signature: general_purpose::STANDARD.encode(&proof.signature),
            round: proof.round,
            block_id_hash: hex::encode(&proof.block_id_hash),
            quorum_type: proof.quorum_type,
        }
    }
}