use crate::{BlsModule, ProtocolError};
use std::sync::Arc;

use crate::identity::validation::PublicKeysValidator;
use crate::identity::IdentityFacade;
use crate::state_repository::StateRepositoryLike;
use crate::state_transition::StateTransitionFacade;
use crate::version::{ProtocolVersionValidator, COMPATIBILITY_MAP, LATEST_VERSION};

pub struct DashPlatformProtocol<SR: StateRepositoryLike + Clone, BLS: BlsModule + Clone> {
    /// Version of protocol
    pub protocol_version: u32,
    /// Public facing facades to interact with the library
    pub identities: IdentityFacade<BLS>,
    // Public facing facades to interact with the library
    pub state_transitions: StateTransitionFacade<SR, BLS>,
    /// State Repository provides the access to the stateful validation
    pub state_repository: SR,
}

/// DashPlatformProtocol is the main interface of the library used to perform validation
/// and creating of different data structures
impl<SR: StateRepositoryLike + Clone, BLS: BlsModule + Clone> DashPlatformProtocol<SR, BLS> {
    pub fn new(
        options: DPPOptions,
        state_repository: SR,
        bls_validator: BLS,
    ) -> Result<Self, ProtocolError>
    where
        SR: StateRepositoryLike + Clone,
        BLS: BlsModule + Clone,
    {
        let current_protocol_version = options.current_protocol_version.unwrap_or(LATEST_VERSION);

        let protocol_version_validator = Arc::new(ProtocolVersionValidator::new(
            current_protocol_version,
            LATEST_VERSION,
            COMPATIBILITY_MAP.clone(),
        ));

        let public_keys_validator = Arc::new(PublicKeysValidator::new(bls_validator.clone())?);

        Ok(Self {
            state_repository: state_repository.clone(),
            protocol_version: current_protocol_version,
            identities: IdentityFacade::new(
                current_protocol_version,
                protocol_version_validator.clone(),
                public_keys_validator,
            )?,
            state_transitions: StateTransitionFacade::new(
                state_repository,
                bls_validator,
                protocol_version_validator,
            )?,
        })
    }

    pub fn identities(&self) -> &IdentityFacade<BLS> {
        &self.identities
    }
}

#[derive(Default, Clone)]
pub struct DPPOptions {
    pub current_protocol_version: Option<u32>,
}
