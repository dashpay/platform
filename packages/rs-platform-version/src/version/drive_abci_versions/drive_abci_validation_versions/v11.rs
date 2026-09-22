use crate::version::drive_abci_versions::drive_abci_validation_versions::v10::DRIVE_ABCI_VALIDATION_VERSIONS_V10;
use crate::version::drive_abci_versions::drive_abci_validation_versions::{
    DriveAbciDocumentsStateTransitionValidationVersions,
    DriveAbciStateTransitionValidationVersions, DriveAbciValidationVersions,
};

/// Protocol v15 validation versions: the delete and erase lifecycle of
/// keep-history documents.
///
/// The batch generations selected by earlier protocol versions stay selected;
/// the erase kind is carried by the shipped batch and reaches them through the
/// arms appended for it, which the basic-structure gate keeps unreachable
/// below this version.
///
/// * delete structure v2 admits a delete of a keep-history document and
///   refuses one whose type carries a contested index; delete state v1 refuses
///   a delete of a document that is already deleted or erasing as a paid
///   consensus error; create state v3 refuses the id of such a document;
/// * the erase generations validate the erase itself and read the lifecycle
///   it acts on.
pub const DRIVE_ABCI_VALIDATION_VERSIONS_V11: DriveAbciValidationVersions =
    DriveAbciValidationVersions {
        state_transitions: DriveAbciStateTransitionValidationVersions {
            batch_state_transition: DriveAbciDocumentsStateTransitionValidationVersions {
                document_delete_transition_structure_validation: 2,
                document_create_transition_state_validation: 3,
                document_erase_transition_structure_validation: Some(0),
                document_delete_transition_state_validation: 1,
                document_erase_transition_state_validation: Some(0),
                fetch_keep_history_document_lifecycle: Some(0),
                ..DRIVE_ABCI_VALIDATION_VERSIONS_V10
                    .state_transitions
                    .batch_state_transition
            },
            ..DRIVE_ABCI_VALIDATION_VERSIONS_V10.state_transitions
        },
        ..DRIVE_ABCI_VALIDATION_VERSIONS_V10
    };
