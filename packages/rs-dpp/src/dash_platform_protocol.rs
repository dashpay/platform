use std::sync::Arc;

use crate::errors::DashPlatformProtocolInitError;
use crate::identity::IdentityFacade;
use crate::version::LATEST_VERSION;

pub struct DashPlatformProtocol<SR> {
    /// Version of protocol
    pub protocol_version: u32,
    /// Public facing facades to interact with the library
    pub identities: IdentityFacade,
    /// State Repository provides the access to the stateful validation
    pub state_repository: SR,
}

/// DashPlatformProtocol is the main interface of the library used to perform validation
/// and creating of different data structures
impl<SR> DashPlatformProtocol<SR> {
    pub fn new(
        options: DPPOptions,
        state_repository: SR,
    ) -> Result<Self, DashPlatformProtocolInitError> {
        let current_protocol_version = options.current_protocol_version.unwrap_or(LATEST_VERSION);

        Ok(Self {
            state_repository,
            protocol_version: current_protocol_version,
            identities: IdentityFacade::new(current_protocol_version)?,
        })
    }

    pub fn identities(&self) -> &IdentityFacade {
        &self.identities
    }
}

#[derive(Default)]
pub struct DPPOptions {
    pub current_protocol_version: Option<u32>,
}
