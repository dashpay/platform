use super::v4::DRIVE_DOCUMENT_METHOD_VERSIONS_V4;
use super::{
    DriveDocumentDeleteMethodVersions, DriveDocumentEstimationCostsMethodVersions,
    DriveDocumentInsertMethodVersions, DriveDocumentMethodVersions,
    DriveDocumentQueryMethodVersions, DriveDocumentUpdateMethodVersions,
};

/// Protocol v15 document methods for per-type keep-history storage.
pub const DRIVE_DOCUMENT_METHOD_VERSIONS_V5: DriveDocumentMethodVersions =
    DriveDocumentMethodVersions {
        query: DriveDocumentQueryMethodVersions {
            fetch_document_history_query: 1,
            fetch_document_history: 1,
            prove_document_history: 1,
            primary_key_path_query: 1,
            ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4.query
        },
        delete: DriveDocumentDeleteMethodVersions {
            remove_reference_for_index_level_for_contract_operations: 2,
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
