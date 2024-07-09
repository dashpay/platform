use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Masternode {pro_tx_hash} voter identity id is incorrect, expected is {expected_voter_identity_id}, provided is {provided_voter_identity_id}")]
#[platform_serialize(unversioned)]
pub struct MasternodeIncorrectVoterIdentityIdError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pro_tx_hash: Identifier,

    expected_voter_identity_id: Identifier,

    provided_voter_identity_id: Identifier,
}

impl MasternodeIncorrectVoterIdentityIdError {
    pub fn new(
        pro_tx_hash: Identifier,
        expected_voter_identity_id: Identifier,
        provided_voter_identity_id: Identifier,
    ) -> Self {
        Self {
            pro_tx_hash,
            expected_voter_identity_id,
            provided_voter_identity_id,
        }
    }

    pub fn pro_tx_hash(&self) -> Identifier {
        self.pro_tx_hash
    }

    pub fn expected_voter_identity_id(&self) -> Identifier {
        self.expected_voter_identity_id
    }

    pub fn provided_voter_identity_id(&self) -> Identifier {
        self.provided_voter_identity_id
    }
}

impl From<MasternodeIncorrectVoterIdentityIdError> for ConsensusError {
    fn from(err: MasternodeIncorrectVoterIdentityIdError) -> Self {
        Self::StateError(StateError::MasternodeIncorrectVoterIdentityIdError(err))
    }
}
