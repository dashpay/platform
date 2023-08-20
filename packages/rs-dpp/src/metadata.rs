use bincode::Encode;
use platform_serialization::de::Decode;
use serde::{Deserialize, Serialize};

use crate::{errors::ProtocolError, prelude::TimestampMillis, util::deserializer::ProtocolVersion};

#[derive(
    Serialize, Deserialize, Encode, Decode, Debug, Default, Clone, Copy, PartialEq, PartialOrd, Eq,
)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    #[serde(default)]
    pub block_height: u64,
    #[serde(default)]
    pub core_chain_locked_height: u64,
    #[serde(default)]
    pub time_ms: TimestampMillis,
    #[serde(default)]
    pub protocol_version: ProtocolVersion,
}

impl std::convert::TryFrom<&str> for Metadata {
    type Error = ProtocolError;

    fn try_from(d: &str) -> Result<Metadata, Self::Error> {
        serde_json::from_str(d).map_err(|e| ProtocolError::EncodingError(e.to_string()))
    }
}
