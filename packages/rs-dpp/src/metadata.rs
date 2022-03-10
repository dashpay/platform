use crate::errors::ProtocolError;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Metadata {
    pub block_height: u64,
    pub core_chain_locked_height: u64,
}

impl std::convert::TryFrom<&str> for Metadata {
    type Error = ProtocolError;

    fn try_from(d: &str) -> Result<Metadata, Self::Error> {
        serde_json::from_str(d).map_err(|e| ProtocolError::EncodingError(e.to_string()))
    }
}
