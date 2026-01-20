use crate::version::drive_abci_versions::drive_abci_validation_versions::v7::DRIVE_ABCI_VALIDATION_VERSIONS_V7;
use crate::version::drive_abci_versions::drive_abci_validation_versions::DriveAbciValidationVersions;

// Introduce document reference validation in document create/replace state validation v2
pub const DRIVE_ABCI_VALIDATION_VERSIONS_V8: DriveAbciValidationVersions =
    DriveAbciValidationVersions {
        state_transitions: crate::version::drive_abci_versions::drive_abci_validation_versions::DriveAbciStateTransitionValidationVersions {
            batch_state_transition: crate::version::drive_abci_versions::drive_abci_validation_versions::DriveAbciDocumentsStateTransitionValidationVersions {
                document_create_transition_state_validation: 2,
                document_replace_transition_state_validation: 2,
                ..DRIVE_ABCI_VALIDATION_VERSIONS_V7.state_transitions.batch_state_transition
            },
            ..DRIVE_ABCI_VALIDATION_VERSIONS_V7.state_transitions
        },
        ..DRIVE_ABCI_VALIDATION_VERSIONS_V7
    };
