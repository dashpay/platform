use crate::identity::IdentityFacade;
use crate::state_transition::state_transition_factory::StateTransitionFactory;

pub struct DashPlatformProtocol {
    /// Version of protocol
    pub protocol_version: u32,
    /// Public facing facades to interact with the library
    pub identities: IdentityFacade,
    pub state_transition: StateTransitionFactory,
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
