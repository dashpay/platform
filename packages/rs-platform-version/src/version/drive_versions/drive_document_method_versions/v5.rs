use crate::version::drive_versions::drive_document_method_versions::v4::DRIVE_DOCUMENT_METHOD_VERSIONS_V4;
use crate::version::drive_versions::drive_document_method_versions::{
    DriveDocumentDeleteMethodVersions, DriveDocumentEstimationCostsMethodVersions,
    DriveDocumentInsertMethodVersions, DriveDocumentMethodVersions,
    DriveDocumentQueryMethodVersions, DriveDocumentUpdateMethodVersions,
};

/// V5 is protocol version 15's document-method table. Relative to V4:
///
/// * `insert_empty_private_document_store_operation` is `Some(0)` (`None` in every earlier
///   table): a contract registering a private document type, or an update adding one, creates
///   the type's private document store at its key `[0]`, sized from the private document store
///   limits of `SYSTEM_LIMITS_V5`. `fetch_private_document_entries` and
///   `prove_private_document_entries` are `Some(0)` too (`None` before): a page of a store's
///   entries can be read and proved, and `append_private_document_entries_operations` is
///   `Some(0)` (`None` before): a `PrivateDocumentCreateFromShieldedPool` appends entries to one.
/// * `insert.add_reference_for_index_level_for_contract_operations` is 1: summable index
///   entries use `AllReferencesWithSumItem` for every type, as delete v2 does, so estimates
///   also change for summable types without history.
/// * `delete.remove_reference_for_index_level_for_contract_operations` is 2: a delete's estimate
///   describes an index's entries for a value (its `[0]` layer) as the references they are, as
///   the insert's does, including the primary-pointer and summable reference shapes.
/// * `estimation_costs.stateless_delete_of_non_tree_for_costs` is 1: a delete's estimate prices
///   each height of the climb with the layer GroveDB deletes in there, not the one above it.
/// * Retained-history writers, readers and estimators use the per-type revision layout.
pub const DRIVE_DOCUMENT_METHOD_VERSIONS_V5: DriveDocumentMethodVersions =
    DriveDocumentMethodVersions {
        document_history_storage_layout: 1,
        query: DriveDocumentQueryMethodVersions {
            fetch_document_history_query: 1,
            fetch_document_history: 1,
            prove_document_history: 1,
            ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4.query
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
        insert_empty_private_document_store_operation: Some(0), // changed: a private document type's store is created with it
        fetch_private_document_entries: Some(0), // changed: a page of a store's entries can be read
        prove_private_document_entries: Some(0), // changed: and proved
        append_private_document_entries_operations: Some(0), // changed: a PrivateDocumentCreateFromShieldedPool appends entries to a store
        delete: DriveDocumentDeleteMethodVersions {
            remove_reference_for_index_level_for_contract_operations: 2, // changed: the entries layer is described as references
            ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4.delete
        },
        estimation_costs: DriveDocumentEstimationCostsMethodVersions {
            add_estimation_costs_for_add_document_to_primary_storage: 1,
            stateless_delete_of_non_tree_for_costs: 1, // changed: each height gets the layer priced there
            ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4.estimation_costs
        },
        ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4
    };
