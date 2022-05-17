use super::{StateTransition, StateTransitionConvert, StateTransitionLike, StateTransitionType};
use crate::util::json_value::JsonValueExt;
use crate::util::{hash, serializer};
use crate::{prelude::ProtocolError, util::json_value::ReplaceWith};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

const PROPERTY_SIGNATURE: &str = "signature";
const PROPERTY_PROTOCOL_VERSION: &str = "protocolVersion";

// The example implementation of generic state transition:
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExampleStateTransition {
    pub protocol_version: u32,
    pub signature: Vec<u8>,
    pub transition_type: StateTransitionType,
}

// implementation of Into is necessary for the example. In normal situation the
// the From trait should be implemented where the [`StateTransition`] is defined
impl Into<StateTransition> for ExampleStateTransition {
    fn into(self) -> StateTransition {
        unimplemented!()
    }
}

impl StateTransitionLike for ExampleStateTransition {
    fn get_protocol_version(&self) -> u32 {
        self.protocol_version
    }

    fn get_signature(&self) -> &Vec<u8> {
        &self.signature
    }

    fn get_type(&self) -> StateTransitionType {
        self.transition_type
    }

    fn calculate_fee(&self) -> Result<u64, ProtocolError> {
        unimplemented!()
    }

    fn set_signature(&mut self, signature: Vec<u8>) {
        self.signature = signature
    }
}

impl StateTransitionConvert for ExampleStateTransition {
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

    fn hash(&self, skip_signature: bool) -> Result<Vec<u8>, ProtocolError> {
        Ok(hash::hash(self.to_buffer(skip_signature)?))
    }
}
