use anyhow::anyhow;
use ciborium::value::Value as CborValue;
use integer_encoding::VarInt;
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
        let (protocol_version, offset) =
            u32::decode_var(buffer.as_ref()).ok_or(ProtocolError::DecodingError(
                "contract cbor could not decode protocol version".to_string(),
            ))?;
        let (_, document_bytes) = buffer.as_ref().split_at(offset);
        let cbor_value: CborValue = ciborium::de::from_reader(document_bytes).map_err(|e| {
            ConsensusError::SerializedObjectParsingError {
                parsing_error: anyhow!("Decode protocol entity: {:#?}", e),
            }
        })?;
        let json_value: JsonValue = serde_json::to_value(cbor_value).unwrap();
        Ok((protocol_version, json_value))
    }
}
