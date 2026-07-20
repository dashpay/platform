use crate::version::drive_abci_versions::drive_abci_validation_versions::{
    DriveAbciDocumentsStateTransitionValidationVersions,
    DriveAbciStateTransitionValidationVersions, DriveAbciValidationVersions,
};

use super::v8::DRIVE_ABCI_VALIDATION_VERSIONS_V8;

/// Protocol v13 validation versions.
///
/// The v1 token validators bind confirmations to stored intent, resolve group
/// authority fail-closed, and validate configuration updates against the
/// configured quorum and prospective configuration.
pub const DRIVE_ABCI_VALIDATION_VERSIONS_V9: DriveAbciValidationVersions =
    DriveAbciValidationVersions {
        state_transitions: DriveAbciStateTransitionValidationVersions {
            batch_state_transition: DriveAbciDocumentsStateTransitionValidationVersions {
                token_base_transition_state_validation: 1,
                token_base_transition_group_action_validation: 1,
                token_config_update_transition_state_validation: 1,
                ..DRIVE_ABCI_VALIDATION_VERSIONS_V8
                    .state_transitions
                    .batch_state_transition
            },
            ..DRIVE_ABCI_VALIDATION_VERSIONS_V8.state_transitions
        },
        ..DRIVE_ABCI_VALIDATION_VERSIONS_V8
    };
