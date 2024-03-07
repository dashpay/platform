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
pub const MAX_MISSING_IDENTITY_REVISIONS: u64 = 24;
pub const MISSING_IDENTITY_REVISIONS_MAX_BYTES: u64 = MAX_MISSING_IDENTITY_REVISIONS;
pub const IDENTITY_NONCE_VALUE_FILTER_MAX_BYTES: u64 = 40;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
/// The result of the merge of the identity contract nonce
pub enum MergeIdentityNonceResult {
    /// The nonce is an invalid value
    /// This could be 0
    InvalidNonce,
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
        f.write_str(self.error_message().unwrap_or("no error"))
    }
}

impl MergeIdentityNonceResult {
    /// Gives a result from the enum
    pub fn error_message(&self) -> Option<&'static str> {
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
            MergeIdentityNonceResult::InvalidNonce => Some("nonce is an invalid value"),
        }
    }

    /// Is this result an error?
    pub fn is_error(&self) -> bool {
        !matches!(self, MergeIdentityNonceResult::MergeIdentityNonceSuccess(_))
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
                current_identity_nonce: None,
                setting_identity_nonce: new_revision_nonce,
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
    match actual_existing_revision.cmp(&new_revision_nonce) {
        std::cmp::Ordering::Equal => {
            // we were not able to update the revision as it is the same as we already had
            return SimpleConsensusValidationResult::new_with_error(ConsensusError::StateError(
                StateError::InvalidIdentityNonceError(InvalidIdentityNonceError {
                    identity_id,
                    current_identity_nonce: Some(existing_nonce),
                    setting_identity_nonce: new_revision_nonce,
                    error: MergeIdentityNonceResult::NonceAlreadyPresentAtTip,
                }),
            ));
        }
        std::cmp::Ordering::Less => {
            if new_revision_nonce - actual_existing_revision > MISSING_IDENTITY_REVISIONS_MAX_BYTES {
                // we are too far away from the actual revision
                return SimpleConsensusValidationResult::new_with_error(ConsensusError::StateError(
                    StateError::InvalidIdentityNonceError(InvalidIdentityNonceError {
                        identity_id,
                        current_identity_nonce: Some(existing_nonce),
                        setting_identity_nonce: new_revision_nonce,
                        error: MergeIdentityNonceResult::NonceTooFarInFuture,
                    }),
                ));
            }
        }
        std::cmp::Ordering::Greater => {
            let previous_revision_position_from_top = actual_existing_revision - new_revision_nonce;
            if previous_revision_position_from_top > MISSING_IDENTITY_REVISIONS_MAX_BYTES {
                // we are too far away from the actual revision
                return SimpleConsensusValidationResult::new_with_error(ConsensusError::StateError(
                    StateError::InvalidIdentityNonceError(InvalidIdentityNonceError {
                        identity_id,
                        current_identity_nonce: Some(existing_nonce),
                        setting_identity_nonce: new_revision_nonce,
                        error: MergeIdentityNonceResult::NonceTooFarInPast,
                    }),
                ));
            } else {
                let old_missing_revisions = existing_nonce & MISSING_IDENTITY_REVISIONS_FILTER;
                let old_revision_already_set = if old_missing_revisions == 0 {
                    true
                } else {
                    let byte_to_unset = 1
                        << (previous_revision_position_from_top - 1
                        + IDENTITY_NONCE_VALUE_FILTER_MAX_BYTES);
                    old_missing_revisions | byte_to_unset != old_missing_revisions
                };

                if old_revision_already_set {
                    return SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::StateError(StateError::InvalidIdentityNonceError(
                            InvalidIdentityNonceError {
                                identity_id,
                                current_identity_nonce: Some(existing_nonce),
                                setting_identity_nonce: new_revision_nonce,
                                error: MergeIdentityNonceResult::NonceAlreadyPresentInPast(
                                    previous_revision_position_from_top,
                                ),
                            },
                        )),
                    );
                }
            }
        }
    }
    SimpleConsensusValidationResult::new()
}

#[cfg(test)]
mod tests {
    use crate::consensus::state::state_error::StateError;
    use crate::consensus::ConsensusError;
    use crate::identity::identity_nonce::{
        validate_identity_nonce_update, MergeIdentityNonceResult,
    };
    use platform_value::Identifier;

    #[test]
    fn validate_identity_nonce_not_changed() {
        let tip = 50;
        let new_nonce = tip;
        let identity_id = Identifier::default();
        let result = validate_identity_nonce_update(tip, new_nonce, identity_id);

        let Some(ConsensusError::StateError(StateError::InvalidIdentityNonceError(e))) =
            result.errors.first()
        else {
            panic!("expected state error");
        };
        assert_eq!(e.error, MergeIdentityNonceResult::NonceAlreadyPresentAtTip);
    }

    #[test]
    fn validate_identity_nonce_update_too_far_in_past() {
        let tip = 50;
        let new_nonce = tip - 25;
        let identity_id = Identifier::default();
        let result = validate_identity_nonce_update(tip, new_nonce, identity_id);

        let Some(ConsensusError::StateError(StateError::InvalidIdentityNonceError(e))) =
            result.errors.first()
        else {
            panic!("expected state error");
        };
        assert_eq!(e.error, MergeIdentityNonceResult::NonceTooFarInPast);
    }

    #[test]
    fn validate_identity_nonce_update_too_far_in_future() {
        let tip = 50;
        let new_nonce = tip + 25;
        let identity_id = Identifier::default();
        let result = validate_identity_nonce_update(tip, new_nonce, identity_id);

        let Some(ConsensusError::StateError(StateError::InvalidIdentityNonceError(e))) =
            result.errors.first()
        else {
            panic!("expected state error");
        };
        assert_eq!(e.error, MergeIdentityNonceResult::NonceTooFarInFuture);
    }

    #[test]
    fn validate_identity_nonce_update_already_in_past_no_missing_in_nonce() {
        let tip = 50;
        let new_nonce = tip - 24;
        let identity_id = Identifier::default();
        let result = validate_identity_nonce_update(tip, new_nonce, identity_id);

        let Some(ConsensusError::StateError(StateError::InvalidIdentityNonceError(e))) =
            result.errors.first()
        else {
            panic!("expected state error");
        };
        assert_eq!(
            e.error,
            MergeIdentityNonceResult::NonceAlreadyPresentInPast(24)
        );
    }

    #[test]
    fn validate_identity_nonce_update_already_in_past_some_missing_in_nonce() {
        let tip = 50 | 0x0FFF000000000000;
        let new_nonce = 50 - 24;
        let identity_id = Identifier::default();
        let result = validate_identity_nonce_update(tip, new_nonce, identity_id);

        let Some(ConsensusError::StateError(StateError::InvalidIdentityNonceError(e))) =
            result.errors.first()
        else {
            panic!("expected state error");
        };
        assert_eq!(
            e.error,
            MergeIdentityNonceResult::NonceAlreadyPresentInPast(24)
        );
    }

    #[test]
    fn validate_identity_nonce_update_not_in_past_some_missing_in_nonce() {
        let tip = 50 | 0x0FFF000000000000;
        let new_nonce = 50 - 20;
        let identity_id = Identifier::default();
        let result = validate_identity_nonce_update(tip, new_nonce, identity_id);

        assert!(result.errors.is_empty())
    }

    #[test]
    fn validate_identity_nonce_in_close_future() {
        let tip = 50 | 0x0FFF000000000000;
        let new_nonce = 50 + 24;
        let identity_id = Identifier::default();
        let result = validate_identity_nonce_update(tip, new_nonce, identity_id);

        assert!(result.errors.is_empty())
    }
}
