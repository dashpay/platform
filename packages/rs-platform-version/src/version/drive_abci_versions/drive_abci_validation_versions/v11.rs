use super::v10::DRIVE_ABCI_VALIDATION_VERSIONS_V10;
use super::{
    DriveAbciDocumentsStateTransitionValidationVersions, DriveAbciStateTransitionValidationVersion,
    DriveAbciStateTransitionValidationVersions, DriveAbciValidationVersions,
};

/// Protocol 15 enables token pool proofs and payments, retaining protocol 14 validation.
pub const DRIVE_ABCI_VALIDATION_VERSIONS_V11: DriveAbciValidationVersions =
    DriveAbciValidationVersions {
        state_transitions: DriveAbciStateTransitionValidationVersions {
            batch_state_transition: DriveAbciDocumentsStateTransitionValidationVersions {
                document_base_transition_state_validation: 1,
                ..DRIVE_ABCI_VALIDATION_VERSIONS_V10
                    .state_transitions
                    .batch_state_transition
            },
            token_shielded_transfer_with_shielded_fee_state_transition:
                DriveAbciStateTransitionValidationVersion {
                    basic_structure: Some(0),
                    ..DRIVE_ABCI_VALIDATION_VERSIONS_V10
                        .state_transitions
                        .token_shielded_transfer_with_shielded_fee_state_transition
                },
            token_unshield_with_shielded_fee_state_transition:
                DriveAbciStateTransitionValidationVersion {
                    basic_structure: Some(0),
                    ..DRIVE_ABCI_VALIDATION_VERSIONS_V10
                        .state_transitions
                        .token_unshield_with_shielded_fee_state_transition
                },
            token_purchase_from_shielded_pool_state_transition:
                DriveAbciStateTransitionValidationVersion {
                    basic_structure: Some(0),
                    ..DRIVE_ABCI_VALIDATION_VERSIONS_V10
                        .state_transitions
                        .token_purchase_from_shielded_pool_state_transition
                },
            ..DRIVE_ABCI_VALIDATION_VERSIONS_V10.state_transitions
        },
        validate_shielded_proof: 2,
        ..DRIVE_ABCI_VALIDATION_VERSIONS_V10
    };
