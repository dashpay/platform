use crate::version::drive_versions::drive_document_method_versions::v4::DRIVE_DOCUMENT_METHOD_VERSIONS_V4;
use crate::version::drive_versions::drive_document_method_versions::{
    DriveDocumentDeleteMethodVersions, DriveDocumentEstimationCostsMethodVersions,
    DriveDocumentInsertMethodVersions, DriveDocumentMethodVersions,
    DriveDocumentUpdateMethodVersions,
};

/// V5 is protocol version 15's document-method table. Relative to V4:
///
/// * `insert_empty_private_document_store_operation` is `Some(0)` (`None` in every earlier
///   table): a contract registering a private document type, or an update adding one, creates
///   the type's private document store at its key `[0]`, sized from the private document store
///   limits of `SYSTEM_LIMITS_V5`. `fetch_private_document_entries` and
///   `prove_private_document_entries` are `Some(0)` too (`None` before): a page of a store's
///   entries can be read and proved.
/// * `delete.remove_reference_for_index_level_for_contract_operations` is 2: a delete's estimate
///   describes an index's entries for a value (its `[0]` layer) as the references they are, as
///   the insert's does, not as subtrees.
/// * `estimation_costs.stateless_delete_of_non_tree_for_costs` is 1: a delete's estimate prices
///   each height of the climb with the layer GroveDB deletes in there, not the one above it.
/// * The six fee-returning wrappers (`add_document_for_contract`,
///   `delete_document_for_contract`, `delete_document_for_contract_id`,
///   `update_document_for_contract`, `update_document_for_contract_id` and
///   `update_document_with_serialization_for_contract`) are bumped to `1`: when the caller
///   passes no transaction they write and price inside one owned transaction and commit it
///   only once `Drive::calculate_fee` succeeded. Pricing an owner-attributed storage
///   removal without the fee history is an error from this version, and generation 0
///   committed the write before that error surfaced. The operation builders and the
///   `_apply_and_add_to_operations` methods every production caller uses through
///   `apply_drive_operations` are unchanged.
pub const DRIVE_DOCUMENT_METHOD_VERSIONS_V5: DriveDocumentMethodVersions =
    DriveDocumentMethodVersions {
        update: DriveDocumentUpdateMethodVersions {
            update_document_for_contract: 1,
            update_document_for_contract_id: 1,
            update_document_with_serialization_for_contract: 1,
            ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4.update
        },
        insert: DriveDocumentInsertMethodVersions {
            add_document_for_contract: 1,
            ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4.insert
        },
        insert_empty_private_document_store_operation: Some(0), // changed: a private document type's store is created with it
        fetch_private_document_entries: Some(0), // changed: a page of a store's entries can be read
        prove_private_document_entries: Some(0), // changed: and proved
        delete: DriveDocumentDeleteMethodVersions {
            remove_reference_for_index_level_for_contract_operations: 2, // changed: the entries layer is described as references
            delete_document_for_contract: 1,
            delete_document_for_contract_id: 1,
            ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4.delete
        },
        estimation_costs: DriveDocumentEstimationCostsMethodVersions {
            stateless_delete_of_non_tree_for_costs: 1, // changed: each height gets the layer priced there
            ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4.estimation_costs
        },
        ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4
    };
