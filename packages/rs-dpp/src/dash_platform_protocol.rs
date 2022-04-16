use crate::errors::DashPlatformProtocolInitError;
use crate::identity::validation::{PublicKeysValidator};
use crate::identity::IdentityFacade;
use crate::version::{ProtocolVersionValidator, COMPATIBILITY_MAP, LATEST_VERSION};
use std::sync::Arc;

pub struct DashPlatformProtocol {
    // Public facing facades to interact with the library
    pub identities: IdentityFacade,
}

/// DashPlatformProtocol is the main interface of the library used to perform validation
/// and creating of different data structures
impl DashPlatformProtocol {
    pub fn new(options: DPPOptions) -> Result<Self, DashPlatformProtocolInitError> {
        let current_protocol_version = options.current_protocol_version.unwrap_or(LATEST_VERSION);

        let protocol_version_validator = Arc::new(ProtocolVersionValidator::new(
            current_protocol_version,
            LATEST_VERSION,
            COMPATIBILITY_MAP.clone(),
        ));

        let public_keys_validator = Arc::new(PublicKeysValidator::new()?);

        Ok(Self {
            identities: IdentityFacade::new(
                protocol_version_validator.clone(),
                public_keys_validator.clone(),
            )?,
        })
    }

    pub fn identities(&self) -> &IdentityFacade {
        &self.identities
    }
}

pub struct DPPOptions {
    current_protocol_version: Option<u64>,
}

impl Default for DPPOptions {
    fn default() -> Self {
        Self {
            current_protocol_version: None,
        }
    }
}
