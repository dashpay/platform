use anyhow::anyhow;
use ciborium::value::Value as CborValue;
use std::convert::TryInto;

use platform_value::Value;

use crate::util::deserializer;
use crate::util::deserializer::SplitProtocolVersionOutcome;
use crate::{errors::consensus::ConsensusError, errors::ProtocolError};

#[derive(Default, Clone, Copy)]
pub struct DecodeProtocolEntity {}

impl DecodeProtocolEntity {
    pub fn decode_protocol_entity(buffer: impl AsRef<[u8]>) -> Result<(u32, Value), ProtocolError> {
        let SplitProtocolVersionOutcome {
            protocol_version,
            main_message_bytes: document_bytes,
            ..
        } = deserializer::split_protocol_version(buffer.as_ref())?;

        let cbor_value: CborValue = ciborium::de::from_reader(document_bytes).map_err(|e| {
            ConsensusError::SerializedObjectParsingError {
                parsing_error: anyhow!("Decode protocol entity: {:#?}", e),
            }
        })?;

        Ok((
            protocol_version,
            cbor_value.try_into().map_err(ProtocolError::ValueError)?,
        ))
    }
}
