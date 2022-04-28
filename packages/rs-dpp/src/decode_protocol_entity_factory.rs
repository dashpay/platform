use anyhow::anyhow;
use serde_json::Value as JsonValue;

use crate::{
    errors::ProtocolError, mocks::ConsensusError, util::deserializer::get_protocol_version,
};

#[derive(Default)]
pub struct DecodeProtocolEntityFactory {}

impl DecodeProtocolEntityFactory {
    pub fn decode_protocol_entity(
        &self,
        buffer: impl AsRef<[u8]>,
    ) -> Result<(u32, JsonValue), ProtocolError> {
        let (protocol_bytes, document_bytes) = buffer.as_ref().split_at(4);

        let protocol_version = get_protocol_version(protocol_bytes)?;

        let json_value: JsonValue = ciborium::de::from_reader(document_bytes).map_err(|e| {
            ConsensusError::SerializedObjectParsingError {
                parsing_error: anyhow!("{}", e),
            }
        })?;
        Ok((protocol_version, json_value))
    }
}
