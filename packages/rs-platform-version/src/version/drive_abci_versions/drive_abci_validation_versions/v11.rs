use super::v10::DRIVE_ABCI_VALIDATION_VERSIONS_V10;
use super::{
    DriveAbciStateTransitionValidationVersion, DriveAbciStateTransitionValidationVersions,
    DriveAbciValidationVersions,
};

/// Drive ABCI validation versions 11, introduced in protocol v15. Differs from v10 in:
/// - the identity update `advanced_structure`, 1 rather than 0: an identity update whose added
///   key's proof of possession does not verify (a key whose public key data does not decode fails
///   it too) is refused without a fee and without using the identity's nonce, instead of being
///   charged. The identity's signature does not cover the proofs of possession of the keys it
///   adds. A revision mismatch is still charged.
/// - the masternode vote `transform_into_action`, 1 rather than 0: a vote on a yes/no poll looks up
///   the masternode's previous answer on that poll.
///
/// v10 remains unchanged for `PROTOCOL_VERSION_14` chain replay.
pub const DRIVE_ABCI_VALIDATION_VERSIONS_V11: DriveAbciValidationVersions =
    DriveAbciValidationVersions {
        state_transitions: DriveAbciStateTransitionValidationVersions {
            identity_update_state_transition: DriveAbciStateTransitionValidationVersion {
                advanced_structure: Some(1), // added key proof of possession failures are refused unpaid
                ..DRIVE_ABCI_VALIDATION_VERSIONS_V10
                    .state_transitions
                    .identity_update_state_transition
            },
            masternode_vote_state_transition: DriveAbciStateTransitionValidationVersion {
                transform_into_action: 1, // a vote on a yes/no poll looks up the masternode's previous answer
                ..DRIVE_ABCI_VALIDATION_VERSIONS_V10
                    .state_transitions
                    .masternode_vote_state_transition
            },
            ..DRIVE_ABCI_VALIDATION_VERSIONS_V10.state_transitions
        },
        ..DRIVE_ABCI_VALIDATION_VERSIONS_V10
    };
