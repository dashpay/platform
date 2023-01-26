use std::convert::TryInto;

use anyhow::anyhow;
use integer_encoding::VarInt;
use serde_json::{Map, Number, Value as JsonValue};

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

pub fn get_protocol_version(version_bytes: &[u8]) -> Result<u32, ProtocolError> {
    u32::decode_var(version_bytes)
        .ok_or(
            ConsensusError::ProtocolVersionParsingError {
                parsing_error: anyhow!("length is not 4 bytes"),
            }
            .into(),
        )
        .map(|(protocol_version, _size)| protocol_version)
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
        serializer.serialize_str(&base64::encode(&buffer))
    }
}
