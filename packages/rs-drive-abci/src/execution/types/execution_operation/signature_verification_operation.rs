use crate::error::Error;
use crate::execution::types::execution_operation::OperationLike;
use dpp::fee::Credits;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{KeyType, PartialIdentity};
use dpp::state_transition::identity_create_transition::accessors::IdentityCreateTransitionAccessorsV0;
use dpp::state_transition::identity_update_transition::accessors::IdentityUpdateTransitionAccessorsV0;
use dpp::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters;
use dpp::state_transition::{StateTransition, StateTransitionIdentitySigned};
use dpp::version::PlatformVersion;
use drive::state_transition_action::StateTransitionAction;
use sha2::digest::typenum::op;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignatureVerificationOperation {
    pub signature_type: KeyType,
}

impl SignatureVerificationOperation {
    pub fn new(signature_type: KeyType) -> Self {
        Self { signature_type }
    }
}

impl OperationLike for SignatureVerificationOperation {
    fn processing_cost(&self, platform_version: &PlatformVersion) -> Result<Credits, Error> {
        Ok(self
            .signature_type
            .signature_verify_cost(platform_version)?)
    }

    fn storage_cost(&self, _platform_version: &PlatformVersion) -> Result<Credits, Error> {
        Ok(0)
    }
}
