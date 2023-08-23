use crate::consensus::basic::decode::SerializedObjectParsingError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use std::sync::Arc;

use crate::errors::DashPlatformProtocolInitError;
use crate::identity::IdentityFacade;
use crate::serialization::PlatformDeserializable;
use crate::state_transition::StateTransition;
use crate::version::LATEST_VERSION;
use crate::ProtocolError;

pub struct DashPlatformProtocol {
    /// Version of protocol
    pub protocol_version: u32,
    /// Public facing facades to interact with the library
    pub identities: IdentityFacade,
    pub state_transition: StateTransitionFactory,
}

#[derive(Clone)]
pub struct StateTransitionFactory;

impl StateTransitionFactory {
    pub fn create_from_buffer(buffer: &[u8]) -> Result<StateTransition, ProtocolError> {
        StateTransition::deserialize(buffer).map_err(|e| {
            ConsensusError::BasicError(BasicError::SerializedObjectParsingError(
                SerializedObjectParsingError::new(format!("Decode protocol entity: {:#?}", e)),
            ))
            .into()
        })
    }
}

/// DashPlatformProtocol is the main interface of the library used to perform validation
/// and creating of different data structures
impl DashPlatformProtocol {
    pub fn new(protocol_version: u32) -> Self {
        Self {
            protocol_version,
            identities: IdentityFacade::new(protocol_version),
            state_transition: StateTransitionFactory {},
        }
    }

    pub fn identities(&self) -> &IdentityFacade {
        &self.identities
    }
    pub fn state_transition(&self) -> &StateTransitionFactory {
        &self.state_transition
    }
}
