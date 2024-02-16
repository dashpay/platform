use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use std::fmt::{Display, Formatter};
use thiserror::Error;

use crate::prelude::{Identifier, IdentityNonce, Revision};

use crate::identity::identity_nonce::MergeIdentityNonceResult;
use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[platform_serialize(unversioned)]
pub struct InvalidIdentityNonceError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub identity_id: Identifier,
    pub current_identity_contract_nonce: Option<IdentityNonce>,
    pub setting_identity_contract_nonce: IdentityNonce,
    pub error: MergeIdentityNonceResult,
}
impl Display for InvalidIdentityNonceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        // Pre-calculate the `current_identity_contract_nonce` value
        let current_nonce = self
            .current_identity_contract_nonce
            .as_ref()
            .unwrap_or(&Default::default()) // Assuming `IdentityContractNonce` implements `Default`
            .to_string(); // Assuming `IdentityContractNonce` implements `ToString` or has a similar method for representation

        // Format the error message with pre-calculated `current_nonce`
        write!(f, "Identity {} is trying to set an invalid identity contract nonce. The current identity contract nonce is {}, we are setting {}, error is {}", self.identity_id, current_nonce, self.setting_identity_contract_nonce, self.error)
    }
}

impl InvalidIdentityNonceError {
    pub fn new(
        identity_id: Identifier,
        current_identity_contract_nonce: Option<IdentityNonce>,
        setting_identity_contract_nonce: IdentityNonce,
        error: MergeIdentityNonceResult,
    ) -> Self {
        Self {
            identity_id,
            current_identity_contract_nonce,
            setting_identity_contract_nonce,
            error,
        }
    }

    pub fn identity_id(&self) -> &Identifier {
        &self.identity_id
    }
    pub fn current_identity_contract_nonce(&self) -> Option<&IdentityNonce> {
        self.current_identity_contract_nonce.as_ref()
    }

    pub fn setting_identity_contract_nonce(&self) -> &IdentityNonce {
        &self.setting_identity_contract_nonce
    }

    pub fn error(&self) -> &MergeIdentityNonceResult {
        &self.error
    }
}
impl From<InvalidIdentityNonceError> for ConsensusError {
    fn from(err: InvalidIdentityNonceError) -> Self {
        Self::StateError(StateError::InvalidIdentityNonceError(err))
    }
}
