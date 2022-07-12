use serde::{Deserialize, Serialize};

use crate::prelude::ProtocolError;

use super::{StateTransition, StateTransitionConvert, StateTransitionLike, StateTransitionType};

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

/// To implement the StateTransitionConvert is enough to implement the _property_paths methods
/// The rest of the methods will be automatically implemented as in the blanked implementation
impl StateTransitionConvert for ExampleStateTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![]
    }
    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![]
    }
    fn binary_property_paths() -> Vec<&'static str> {
        vec![]
    }
}
