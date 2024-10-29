use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::identity::core_script::CoreScript;

use crate::identity::KeyID;
use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Withdrawal output script not allowed when signing with owner key {key_id}")]
#[platform_serialize(unversioned)]
pub struct WithdrawalOutputScriptNotAllowedWhenSigningWithOwnerKeyError {
    output_script: CoreScript,
    key_id: KeyID,
}

/*

DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

*/

impl WithdrawalOutputScriptNotAllowedWhenSigningWithOwnerKeyError {
    pub fn new(output_script: CoreScript, key_id: KeyID) -> Self {
        Self {
            output_script,
            key_id,
        }
    }

    pub fn output_script(&self) -> &CoreScript {
        &self.output_script
    }

    pub fn key_id(&self) -> KeyID {
        self.key_id
    }
}
impl From<WithdrawalOutputScriptNotAllowedWhenSigningWithOwnerKeyError> for ConsensusError {
    fn from(err: WithdrawalOutputScriptNotAllowedWhenSigningWithOwnerKeyError) -> Self {
        Self::BasicError(
            BasicError::WithdrawalOutputScriptNotAllowedWhenSigningWithOwnerKeyError(err),
        )
    }
}
