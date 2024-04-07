#[cfg(feature = "cbor")]
use crate::consensus::basic::decode::ProtocolVersionParsingError;
#[cfg(feature = "cbor")]
use crate::consensus::basic::BasicError;
#[cfg(feature = "cbor")]
use crate::consensus::ConsensusError;
use integer_encoding::VarInt;
use platform_version::version::FeatureVersion;
use serde_json::{Map, Number, Value as JsonValue};

use crate::errors::ProtocolError;

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
            ProtocolError::UnknownProtocolVersionError(
                "protocol version could not be decoded as a varint".to_string(),
            )
        })
        .map(|(protocol_version, _size)| protocol_version)
}

/// The outcome of splitting a message that has a protocol version
pub struct SplitFeatureVersionOutcome<'a> {
    /// The protocol version
    pub feature_version: FeatureVersion,
    /// The protocol version size
    pub protocol_version_size: usize,
    /// The main message bytes of the protocol version
    pub main_message_bytes: &'a [u8],
}

#[cfg(feature = "cbor")]
pub fn split_cbor_feature_version(
    message_bytes: &[u8],
) -> Result<SplitFeatureVersionOutcome, ProtocolError> {
    let (feature_version, protocol_version_size) =
        u16::decode_var(message_bytes).ok_or(ConsensusError::BasicError(
            BasicError::ProtocolVersionParsingError(ProtocolVersionParsingError::new(
                "protocol version could not be decoded as a varint".to_string(),
            )),
        ))?;

    // We actually encode protocol version as is. get method of protocol version always expects
    // protocol version to be at least 1, an it will give back version 0 if 1 is passed.
    let (_, main_message_bytes) = message_bytes.split_at(protocol_version_size);

    Ok(SplitFeatureVersionOutcome {
        feature_version,
        protocol_version_size,
        main_message_bytes,
    })
}

pub mod serde_entropy {
    use base64::prelude::BASE64_STANDARD;
    use base64::Engine;
    use std::convert::TryInto;

    use serde::{Deserialize, Deserializer, Serializer};

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 32], D::Error> {
        let data: String = Deserialize::deserialize(d)?;
        BASE64_STANDARD
            .decode(&data)
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
        serializer.serialize_str(&BASE64_STANDARD.encode(buffer))
    }
}
