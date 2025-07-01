use serde::{Deserialize, Serialize};

/// Response from the quorums endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuorumsResponse {
    pub success: bool,
    pub data: Vec<QuorumData>,
}

/// Data about a specific quorum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuorumData {
    pub quorum_hash: String,
    pub key: String,
    pub height: u64,
    pub valid_members_count: u32,
}

/// Information about a specific quorum (internal format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuorumInfo {
    pub version: u16,
    pub llmq_type: u32,
    pub quorum_hash: String,
    pub quorum_public_key: String,
    #[serde(rename = "signersCount")]
    pub signers_count: u32,
    pub signers: String,
    #[serde(rename = "validMembersCount")]
    pub valid_members_count: u32,
    #[serde(rename = "validMembers")]
    pub valid_members: String,
    #[serde(rename = "quorumIndex")]
    pub quorum_index: Option<u32>,
}

/// Response from the previous quorums endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviousQuorumsResponse {
    pub success: bool,
    pub data: PreviousQuorumsData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviousQuorumsData {
    pub height: u64,
    pub quorums: Vec<QuorumData>,
}
