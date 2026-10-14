use crate::version::drive_abci_versions::drive_abci_validation_versions::{
    DriveAbciDocumentsStateTransitionValidationVersions,
    DriveAbciStateTransitionValidationVersions, DriveAbciValidationDataTriggerAndBindingVersions,
    DriveAbciValidationDataTriggerVersions, DriveAbciValidationVersions,
};

use super::v10::DRIVE_ABCI_VALIDATION_VERSIONS_V10;

/// Protocol v15 validation versions.
///
/// `create_domain_data_trigger` 2 keeps only the parent domain checks of a DPNS
/// domain create: the DPNS contract v3 (`SYSTEM_DATA_CONTRACT_VERSIONS_V4`)
/// checks the normalized names, the preorder and the identity record with
/// schema keywords.
pub const DRIVE_ABCI_VALIDATION_VERSIONS_V11: DriveAbciValidationVersions =
    DriveAbciValidationVersions {
        state_transitions: DriveAbciStateTransitionValidationVersions {
            batch_state_transition: DriveAbciDocumentsStateTransitionValidationVersions {
                data_triggers: DriveAbciValidationDataTriggerAndBindingVersions {
                    triggers: DriveAbciValidationDataTriggerVersions {
                        create_domain_data_trigger: 2,
                        ..DRIVE_ABCI_VALIDATION_VERSIONS_V10
                            .state_transitions
                            .batch_state_transition
                            .data_triggers
                            .triggers
                    },
                    ..DRIVE_ABCI_VALIDATION_VERSIONS_V10
                        .state_transitions
                        .batch_state_transition
                        .data_triggers
                },
                ..DRIVE_ABCI_VALIDATION_VERSIONS_V10
                    .state_transitions
                    .batch_state_transition
            },
            ..DRIVE_ABCI_VALIDATION_VERSIONS_V10.state_transitions
        },
        ..DRIVE_ABCI_VALIDATION_VERSIONS_V10
    };
