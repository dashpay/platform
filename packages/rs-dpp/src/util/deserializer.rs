use std::convert::TryInto;

use anyhow::anyhow;
use integer_encoding::VarInt;
use serde_json::{Map, Number, Value as JsonValue};

use crate::data_contract::errors::StructureError;
use crate::data_contract::extra::common::check_protocol_version;
use crate::{errors::consensus::ConsensusError, errors::ProtocolError};

pub fn parse_protocol_version(
    protocol_bytes: &[u8],
    json_map: &mut Map<String, JsonValue>,
) -> Result<(), ProtocolError> {
    let protocol_version = get_protocol_version(protocol_bytes)?;

    json_map.insert(
        String::from("$protocolVersion"),
        JsonValue::Number(Number::from(protocol_version)),
    );
    Ok(())
}

/// A protocol version
pub type ProtocolVersion = u32;

pub fn get_protocol_version(version_bytes: &[u8]) -> Result<ProtocolVersion, ProtocolError> {
    u32::decode_var(version_bytes)
        .ok_or_else(|| {
            ConsensusError::ProtocolVersionParsingError {
                parsing_error: anyhow!("length could not be decoded as a varint"),
            }
            .into()
        })
        .map(|(protocol_version, _size)| protocol_version)
}

/// The outcome of splitting a message that has a protocol version
pub struct SplitProtocolVersionOutcome<'a> {
    /// The protocol version
    pub protocol_version: ProtocolVersion,
    /// The protocol version size
    pub protocol_version_size: usize,
    /// The main message bytes of the protocol version
    pub main_message_bytes: &'a [u8],
}

pub fn split_protocol_version(
    message_bytes: &[u8],
) -> Result<SplitProtocolVersionOutcome, ProtocolError> {
    let (protocol_version, protocol_version_size) =
        u32::decode_var(message_bytes).ok_or(ProtocolError::AbstractConsensusError(Box::new(
            ConsensusError::ProtocolVersionParsingError {
                parsing_error: anyhow!("length could not be decoded as a varint"),
            },
        )))?;
    let (_, main_message_bytes) = message_bytes.split_at(protocol_version_size);

    if !check_protocol_version(protocol_version) {
        return Err(ProtocolError::StructureError(
            StructureError::InvalidProtocolVersion("invalid protocol version"),
        ));
    }

    Ok(SplitProtocolVersionOutcome {
        protocol_version,
        protocol_version_size,
        main_message_bytes,
    })
}

pub mod serde_entropy {
    use std::convert::TryInto;

    use serde::{Deserialize, Deserializer, Serializer};

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 32], D::Error> {
        let data: String = Deserialize::deserialize(d)?;
        base64::decode(&data)
            .map_err(|e| {
                serde::de::Error::custom(format!("Unable to decode {}' with base64 - {}", data, e))
            })?
            .try_into()
            .map_err(|_| {
                serde::de::Error::custom(format!(
                    "Unable to convert the '{:?}' into 32 bytes array",
                    data
                ))
            })
    }

    pub fn serialize<S>(buffer: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&base64::encode(buffer))
    }
}
