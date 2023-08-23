use std::sync::Arc;

use crate::errors::DashPlatformProtocolInitError;
use crate::identity::IdentityFacade;
use crate::version::LATEST_VERSION;

pub struct DashPlatformProtocol {
    /// Version of protocol
    pub protocol_version: u32,
    /// Public facing facades to interact with the library
    pub identities: IdentityFacade,
}

/// DashPlatformProtocol is the main interface of the library used to perform validation
/// and creating of different data structures
impl DashPlatformProtocol {
    pub fn new(protocol_version: u32) -> Self {
        Self {
            protocol_version,
            identities: IdentityFacade::new(protocol_version),
        }
    }

    pub fn identities(&self) -> &IdentityFacade {
        &self.identities
    }
}
