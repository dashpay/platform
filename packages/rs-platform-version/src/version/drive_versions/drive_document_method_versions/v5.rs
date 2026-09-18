use super::v4::DRIVE_DOCUMENT_METHOD_VERSIONS_V4;
use super::{
    DriveDocumentDeleteMethodVersions, DriveDocumentEstimationCostsMethodVersions,
    DriveDocumentInsertMethodVersions, DriveDocumentMethodVersions,
    DriveDocumentQueryMethodVersions, DriveDocumentUpdateMethodVersions,
};

/// Protocol v15 document methods for per-type keep-history storage and the
/// delete and erase lifecycle of keep-history documents.
///
/// A delete of a keep-history document removes its current pointer and index
/// references and records a lifecycle entry while the retained revisions stay
/// readable; an erase removes those revisions a chunk at a time. The delete
/// wrappers, the primary-storage removal estimate and the lifecycle read are
/// the generations that know that layout.
pub const DRIVE_DOCUMENT_METHOD_VERSIONS_V5: DriveDocumentMethodVersions =
    DriveDocumentMethodVersions {
        query: DriveDocumentQueryMethodVersions {
            fetch_document_history_query: 1,
            fetch_document_history: 1,
            prove_document_history: 1,
            primary_key_path_query: 1,
            fetch_document_lifecycle: Some(0),
            ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4.query
        },
        delete: DriveDocumentDeleteMethodVersions {
            remove_reference_for_index_level_for_contract_operations: 2,
            add_estimation_costs_for_remove_document_to_primary_storage: 1,
            delete_document_for_contract: 1,
            delete_document_for_contract_id: 1,
            delete_document_for_contract_apply_and_add_to_operations: 1,
            remove_document_from_primary_storage: 1,
            delete_document_for_contract_id_with_named_type_operations: 1,
            delete_document_for_contract_with_named_type_operations: 1,
            delete_document_for_contract_operations: 1,
            erase_document_for_contract_operations: Some(0),
            add_estimation_costs_for_erase_document: Some(0),
            ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4.delete
        },
        insert: DriveDocumentInsertMethodVersions {
            add_document_to_primary_storage: 1,
            add_reference_for_index_level_for_contract_operations: 1,
            ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4.insert
        },
        update: DriveDocumentUpdateMethodVersions {
            update_document_for_contract_operations: 2,
            ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4.update
        },
        estimation_costs: DriveDocumentEstimationCostsMethodVersions {
            add_estimation_costs_for_add_document_to_primary_storage: 1,
            ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4.estimation_costs
        },
        ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4
    };
