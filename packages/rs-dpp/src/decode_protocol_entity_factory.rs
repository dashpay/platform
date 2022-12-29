use anyhow::anyhow;
use ciborium::value::Value as CborValue;
use serde_json::Value as JsonValue;

use crate::{
    errors::consensus::ConsensusError, errors::ProtocolError,
    util::deserializer::get_protocol_version,
};

#[derive(Default, Clone, Copy)]
pub struct DecodeProtocolEntity {}

impl DecodeProtocolEntity {
    pub fn decode_protocol_entity(
        buffer: impl AsRef<[u8]>,
    ) -> Result<(u32, JsonValue), ProtocolError> {
        let (protocol_bytes, document_bytes): (&[u8], &[u8]) = buffer.as_ref().split_at(4);
        let protocol_version = get_protocol_version(protocol_bytes)?;
        let cbor_value: CborValue = ciborium::de::from_reader(document_bytes).map_err(|e| {
            ConsensusError::SerializedObjectParsingError {
                parsing_error: anyhow!("Decode protocol entity: {:#?}", e),
            }
        })?;
        let json_value: JsonValue = serde_json::to_value(cbor_value).unwrap();
        Ok((protocol_version, json_value))
    }
}
