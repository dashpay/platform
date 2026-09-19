use crate::version::drive_abci_versions::drive_abci_validation_versions::v10::DRIVE_ABCI_VALIDATION_VERSIONS_V10;
use crate::version::drive_abci_versions::drive_abci_validation_versions::{
    DriveAbciDocumentsStateTransitionValidationVersions,
    DriveAbciStateTransitionValidationVersions, DriveAbciValidationVersions,
};

/// Protocol v15 validation versions: the delete and erase lifecycle of
/// keep-history documents, carried by batch transition wire format 2.
///
/// Every batch generation bumped here reads a batch through the shell that
/// knows the erase kind and produces or consumes batch action format 1, whose
/// items may include an erase. The generations selected by earlier protocol
/// versions keep the shell and action format of wire formats 0 and 1.
///
/// * `transform_into_action: 2` runs the transformer that sees every wire
///   format and emits action format 1;
/// * `advanced_structure: 1` and `state: 1` validate action format 1;
/// * `revision: 1` and `is_allowed: 1` read nonces and contested creates from
///   a batch of any wire format;
/// * `fetch_documents_for_transitions_of_any_format_knowing_contract_and_document_type:
///   Some(0)` bills like generation 1 of the sibling helper for transitions
///   viewed through the shell of any wire format;
/// * delete structure v2 admits a delete of a keep-history document and
///   refuses one whose type carries a contested index; delete state v1 refuses
///   a delete of a document that is already deleted or erasing as a paid
///   consensus error; create state v3 refuses the id of such a document; the
///   erase generations validate the erase itself and read the lifecycle it
///   acts on.
pub const DRIVE_ABCI_VALIDATION_VERSIONS_V11: DriveAbciValidationVersions =
    DriveAbciValidationVersions {
        state_transitions: DriveAbciStateTransitionValidationVersions {
            batch_state_transition: DriveAbciDocumentsStateTransitionValidationVersions {
                advanced_structure: 1,
                state: 1,
                revision: 1,
                transform_into_action: 2,
                fetch_documents_for_transitions_of_any_format_knowing_contract_and_document_type:
                    Some(0),
                is_allowed: 1,
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
