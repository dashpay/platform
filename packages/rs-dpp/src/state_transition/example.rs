use serde::{Deserialize, Serialize};

use super::{
    state_transition_execution_context::StateTransitionExecutionContext, StateTransition,
    StateTransitionConvert, StateTransitionLike, StateTransitionType,
};

const PROPERTY_SIGNATURE: &str = "signature";
const PROPERTY_PROTOCOL_VERSION: &str = "protocolVersion";

// The example implementation of generic state transition:
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExampleStateTransition {
    pub protocol_version: u32,
    pub signature: Vec<u8>,
    pub transition_type: StateTransitionType,
    #[serde(skip)]
    pub execution_context: StateTransitionExecutionContext,
}

impl From<ExampleStateTransition> for StateTransition {
    fn from(_: ExampleStateTransition) -> Self {
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

    fn set_signature(&mut self, signature: Vec<u8>) {
        self.signature = signature
    }
    fn get_execution_context(&self) -> &StateTransitionExecutionContext {
        &self.execution_context
    }

    fn get_execution_context_mut(&mut self) -> &mut StateTransitionExecutionContext {
        &mut self.execution_context
    }

    fn set_execution_context(&mut self, execution_context: StateTransitionExecutionContext) {
        self.execution_context = execution_context
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
