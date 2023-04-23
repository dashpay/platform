use anyhow::anyhow;
#[cfg(feature = "cbor")]
use ciborium::value::Value as CborValue;
use std::convert::TryInto;

use platform_value::Value;

use crate::serialization_traits::PlatformDeserializable;
use crate::state_transition::{StateTransition, StateTransitionConvert};
use crate::util::deserializer;
use crate::util::deserializer::SplitProtocolVersionOutcome;
use crate::{errors::consensus::ConsensusError, errors::ProtocolError, Convertible};

#[derive(Default, Clone, Copy)]
pub struct DecodeProtocolEntity {}

impl DecodeProtocolEntity {
    pub fn decode_protocol_entity<T: PlatformDeserializable>(
        buffer: impl AsRef<[u8]>,
    ) -> Result<(u32, T), ProtocolError> {
        let SplitProtocolVersionOutcome {
            protocol_version,
            main_message_bytes: document_bytes,
            ..
        } = deserializer::split_protocol_version(buffer.as_ref())?;

        let protocol_entity = T::deserialize(document_bytes).map_err(|e| {
            ConsensusError::SerializedObjectParsingError {
                parsing_error: anyhow!("Decode protocol entity: {:#?}", e),
            }
        })?;

        //todo deal with version

        Ok((protocol_version, protocol_entity))
    }
    pub fn decode_protocol_entity_to_value<T: PlatformDeserializable + Convertible>(
        buffer: impl AsRef<[u8]>,
    ) -> Result<(u32, Value), ProtocolError> {
        let SplitProtocolVersionOutcome {
            protocol_version,
            main_message_bytes: document_bytes,
            ..
        } = deserializer::split_protocol_version(buffer.as_ref())?;

        let protocol_entity = T::deserialize(document_bytes).map_err(|e| {
            ConsensusError::SerializedObjectParsingError {
                parsing_error: anyhow!("Decode protocol entity: {:#?}", e),
            }
        })?;

        Ok((protocol_version, protocol_entity.to_object()?))
    }
}
