use crate::{
    prelude::ProtocolError,
    util::{hash, json_value::ReplaceWith, serializer},
};
use serde_json::Value as JsonValue;

use serde::{Deserialize, Serialize};

use super::StateTransitionType;
use crate::util::json_value::JsonValueExt;

pub const DOCUMENT_TRANSITION_TYPES: [StateTransitionType; 1] =
    [StateTransitionType::DocumentsBatch];

pub const IDENTITY_TRANSITION_TYPE: [StateTransitionType; 2] = [
    StateTransitionType::IdentityCreate,
    StateTransitionType::IdentityTopUp,
];

pub const DATA_CONTRACT_TRANSITION_TYPES: [StateTransitionType; 2] = [
    StateTransitionType::DataContractCreate,
    StateTransitionType::DataContractUpdate,
];

const PROPERTY_SIGNATURE: &str = "signature";
const PROPERTY_PROTOCOL_VERSION: &str = "protocolVersion";

/**
 * @typedef RawStateTransition
 * @property {number} protocolVersion
 * @property {number} type
 * @property {Buffer} [signature]
 */

/**
 * @typedef JsonStateTransition
 * @property {number} protocolVersion
 * @property {number} type
 * @property {string} [signature]
 */

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StateTransitionBase {
    pub protocol_version: u32,
    pub signature: Vec<u8>,
    pub transition_type: StateTransitionType,
}

impl StateTransitionConvert for StateTransitionBase {
    fn to_object(&self, skip_signature: bool) -> Result<JsonValue, ProtocolError> {
        let mut json_value: JsonValue = serde_json::to_value(self)?;
        if skip_signature {
            if let JsonValue::Object(ref mut o) = json_value {
                o.remove(PROPERTY_SIGNATURE);
            }
        }
        Ok(json_value)
    }

    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        let mut json_value: JsonValue = serde_json::to_value(self)?;
        json_value.replace_binary_paths([PROPERTY_SIGNATURE], ReplaceWith::Base64)?;
        Ok(json_value)
    }

    fn to_buffer(&self, skip_signature: bool) -> Result<Vec<u8>, ProtocolError> {
        let mut json_value = self.to_object(skip_signature)?;
        let protocol_version = json_value.remove_u32(PROPERTY_PROTOCOL_VERSION)?;

        serializer::value_to_cbor(json_value, Some(protocol_version))
    }

    fn hash(&self) -> Result<Vec<u8>, ProtocolError> {
        Ok(hash::hash(self.to_buffer(false)?))
    }
}

// TODO remove 'unimplemented' when get rid of state transition mocks
pub trait StateTransitionConvert {
    /// Object is an [`serde_json::Value`] instance that preserves the `Vec<u8>` representation
    /// for Identifiers and binary data
    fn to_object(&self, _skip_signature: bool) -> Result<JsonValue, ProtocolError> {
        unimplemented!()
    }
    /// Object is an [`serde_json::Value`] instance that replaces the binary data with
    ///  - base58 string for Identifiers
    ///  - base64 string for other binary data
    fn to_json(&self) -> Result<JsonValue, ProtocolError> {
        unimplemented!()
    }
    // returns the byte-array representation. It is prefixed by 4 bytes of ProtocolVersion and encoded by CBOR
    fn to_buffer(&self, _skip_signature: bool) -> Result<Vec<u8>, ProtocolError> {
        unimplemented!()
    }
    // hash function is applied to byte-array representation of structure
    fn hash(&self) -> Result<Vec<u8>, ProtocolError> {
        unimplemented!()
    }
}
