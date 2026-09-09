#[cfg(feature = "cbor")]
use crate::consensus::basic::decode::ProtocolVersionParsingError;
#[cfg(feature = "cbor")]
use crate::consensus::basic::BasicError;
#[cfg(feature = "cbor")]
use crate::consensus::ConsensusError;
use integer_encoding::VarInt;
use platform_version::version::FeatureVersion;

use crate::errors::ProtocolError;

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
) -> Result<SplitFeatureVersionOutcome<'_>, ProtocolError> {
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
