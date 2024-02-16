use crate::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use std::fmt::{Debug, Display, Formatter};

use crate::consensus::state::identity::invalid_identity_contract_nonce_error::InvalidIdentityNonceError;
use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::prelude::IdentityNonce;
use crate::validation::SimpleConsensusValidationResult;
use bincode::{Decode, Encode};
use platform_value::Identifier;

pub const IDENTITY_NONCE_VALUE_FILTER: u64 = 0xFFFFFFFFFF;
pub const MISSING_IDENTITY_REVISIONS_FILTER: u64 = 0xFFFFFF0000000000;
pub const MAX_MISSING_IDENTITY_REVISIONS: u64 = 20;
pub const MISSING_IDENTITY_REVISIONS_MAX_BYTES: u64 = MAX_MISSING_IDENTITY_REVISIONS;
pub const IDENTITY_NONCE_VALUE_FILTER_MAX_BYTES: u64 = 40;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
/// The result of the merge of the identity contract nonce
pub enum MergeIdentityNonceResult {
    /// The nonce is too far in the future
    NonceTooFarInFuture,
    /// The nonce is too far in the past
    NonceTooFarInPast,
    /// The nonce is already present at the tip
    NonceAlreadyPresentAtTip,
    /// The nonce is already present in the past
    NonceAlreadyPresentInPast(u64),
    /// The merge is a success
    MergeIdentityNonceSuccess(IdentityNonce),
}

impl Display for MergeIdentityNonceResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.error_message().unwrap_or_else(|| "no error"))
    }
}

impl MergeIdentityNonceResult {
    /// Gives a result from the enum
    pub fn error_message(self) -> Option<&'static str> {
        match self {
            MergeIdentityNonceResult::NonceTooFarInFuture => Some("nonce too far in future"),
            MergeIdentityNonceResult::NonceTooFarInPast => Some("nonce too far in past"),
            MergeIdentityNonceResult::NonceAlreadyPresentAtTip => {
                Some("nonce already present at tip")
            }
            MergeIdentityNonceResult::NonceAlreadyPresentInPast(_) => {
                Some("nonce already present in past")
            }
            MergeIdentityNonceResult::MergeIdentityNonceSuccess(_) => None,
        }
    }
}

pub fn validate_new_identity_nonce(
    new_revision_nonce: IdentityNonce,
    identity_id: Identifier,
) -> SimpleConsensusValidationResult {
    if new_revision_nonce >= MISSING_IDENTITY_REVISIONS_MAX_BYTES {
        // we are too far away from the actual revision
        SimpleConsensusValidationResult::new_with_error(ConsensusError::StateError(
            StateError::InvalidIdentityNonceError(InvalidIdentityNonceError {
                identity_id,
                current_identity_contract_nonce: None,
                setting_identity_contract_nonce: new_revision_nonce,
                error: MergeIdentityNonceResult::NonceTooFarInPast,
            }),
        ))
    } else {
        SimpleConsensusValidationResult::new()
    }
}

pub fn validate_identity_nonce_update(
    existing_nonce: IdentityNonce,
    new_revision_nonce: IdentityNonce,
    identity_id: Identifier,
) -> SimpleConsensusValidationResult {
    let actual_existing_revision = existing_nonce & IDENTITY_NONCE_VALUE_FILTER;
    if actual_existing_revision == new_revision_nonce {
        // we were not able to update the revision as it is the same as we already had
        return SimpleConsensusValidationResult::new_with_error(ConsensusError::StateError(
            StateError::InvalidIdentityNonceError(InvalidIdentityNonceError {
                identity_id,
                current_identity_contract_nonce: Some(existing_nonce),
                setting_identity_contract_nonce: new_revision_nonce,
                error: MergeIdentityNonceResult::NonceAlreadyPresentAtTip,
            }),
        ));
    } else if actual_existing_revision < new_revision_nonce {
        if new_revision_nonce - actual_existing_revision >= MISSING_IDENTITY_REVISIONS_MAX_BYTES {
            // we are too far away from the actual revision
            return SimpleConsensusValidationResult::new_with_error(ConsensusError::StateError(
                StateError::InvalidIdentityNonceError(InvalidIdentityNonceError {
                    identity_id,
                    current_identity_contract_nonce: Some(existing_nonce),
                    setting_identity_contract_nonce: new_revision_nonce,
                    error: MergeIdentityNonceResult::NonceTooFarInFuture,
                }),
            ));
        }
    } else {
        let previous_revision_position_from_top = actual_existing_revision - new_revision_nonce;
        if previous_revision_position_from_top >= MISSING_IDENTITY_REVISIONS_MAX_BYTES {
            // we are too far away from the actual revision
            return SimpleConsensusValidationResult::new_with_error(ConsensusError::StateError(
                StateError::InvalidIdentityNonceError(InvalidIdentityNonceError {
                    identity_id,
                    current_identity_contract_nonce: Some(existing_nonce),
                    setting_identity_contract_nonce: new_revision_nonce,
                    error: MergeIdentityNonceResult::NonceTooFarInPast,
                }),
            ));
        } else {
            let old_missing_revisions = existing_nonce & MISSING_IDENTITY_REVISIONS_FILTER;
            let byte_to_unset = 1
                << (previous_revision_position_from_top - 1
                    + IDENTITY_NONCE_VALUE_FILTER_MAX_BYTES);
            let old_revision_already_set = (old_missing_revisions & !byte_to_unset) > 0;
            if old_revision_already_set {
                return SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::StateError(StateError::InvalidIdentityNonceError(
                        InvalidIdentityNonceError {
                            identity_id,
                            current_identity_contract_nonce: Some(existing_nonce),
                            setting_identity_contract_nonce: new_revision_nonce,
                            error: MergeIdentityNonceResult::NonceAlreadyPresentInPast(
                                previous_revision_position_from_top,
                            ),
                        },
                    )),
                );
            }
        }
    }
    SimpleConsensusValidationResult::new()
}
