use crate::version::drive_versions::drive_document_method_versions::v4::DRIVE_DOCUMENT_METHOD_VERSIONS_V4;
use crate::version::drive_versions::drive_document_method_versions::{
    DriveDocumentDeleteMethodVersions, DriveDocumentEstimationCostsMethodVersions,
    DriveDocumentInsertContestedMethodVersions, DriveDocumentInsertMethodVersions,
    DriveDocumentMethodVersions, DriveDocumentQueryMethodVersions,
    DriveDocumentUpdateMethodVersions,
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
/// * `draw_random_document_properties` is `Some(0)` (`None` in every earlier table): the
///   documents a block created get their `sys.random` properties drawn from the block's random
///   beacon.
/// * `delete.remove_reference_for_index_level_for_contract_operations` is 2: a delete's estimate
///   describes an index's entries for a value (its `[0]` layer) as the references they are, as
///   the insert's does, not as subtrees, and under a summable index that layer and the deleted
///   reference as references with sum items.
/// * `insert.add_reference_for_index_level_for_contract_operations` is 1: an insert's estimate
///   describes a summable index's entries for a value as references with sum items, not plain
///   references.
/// * `estimation_costs.stateless_delete_of_non_tree_for_costs` is 1: a delete's estimate prices
///   each height of the climb with the layer GroveDB deletes in there, not the one above it.
/// * Retained-history writers, readers and estimators use the per-type revision layout.
///   `insert.add_document_for_contract_operations` is 2: a retained id cannot be recreated.
/// * `delete.delete_document_for_contract_operations` is 1: deleting a history document
///   removes its current pointer and indices and writes a lifecycle record. Its primary
///   removal and removal estimate selectors are 1, describing the composite pointer.
/// * `query.fetch_document_lifecycle` is `Some(0)`: reads the current document or its record.
///   `delete.erase_document_for_contract_operations`, `add_estimation_costs_for_erase_document`
///   and `add_estimation_costs_for_lifecycle_record` are `Some(0)`: bounded erasure removes
///   retained revisions and prices the chunk and its lifecycle metadata.
///   `delete.erase_refund_recipient_cost` is `Some(0)`: prices the balance updates caused by erase refunds.
/// * `update.update_document_for_contract_operations` is 2: when an update names a new owner,
///   the references of the indexes it leaves unchanged are rewritten with that owner.
/// * `update.update_document_for_contract_apply_and_add_to_operations` is 1: an update applied
///   on its own hands storage over as one in a batch of drive operations does.
/// * `insert_contested.add_contested_document_for_contract_operations` is 2: a poll's end date
///   entries name whoever the document's storage flags name.
/// * `query.single_document_path_query` is 1: a lookup of a document that may be contested
///   states both branch caps for every type. In a type that keeps history, a single-document
///   lookup at a block time selects the latest retained revision at or before it, and the
///   maybe-contested status is refused at a block time.
/// * `insert.add_indices_for_top_index_level_for_contract_operations` is 3: an insert writes no
///   entry in a bucket of a time range index with a `ttl` that has expired at the block time,
///   as an update already does not.
pub const DRIVE_DOCUMENT_METHOD_VERSIONS_V5: DriveDocumentMethodVersions =
    DriveDocumentMethodVersions {
        document_history_storage_layout: 1,
        query: DriveDocumentQueryMethodVersions {
            fetch_document_lifecycle: Some(0), // changed: reads the current document or lifecycle record
            single_document_path_query: 1, // changed: a temporal lookup selects the latest retained revision
            fetch_document_history_query: 1,
            fetch_document_history: 1,
            prove_document_history: 1,
            ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4.query
        },
        insert: DriveDocumentInsertMethodVersions {
            add_document_for_contract_operations: 2, // changed: refuses reusing a retained document id
            add_document_to_primary_storage: 1,
            add_indices_for_top_index_level_for_contract_operations: 3, // changed: no entry in an expired time range bucket
            add_reference_for_index_level_for_contract_operations: 1, // changed: a summable index's entries layer is described as references with sum items
            ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4.insert
        },
        insert_empty_private_document_store_operation: Some(0), // changed: a private document type's store is created with it
        fetch_private_document_entries: Some(0), // changed: a page of a store's entries can be read
        prove_private_document_entries: Some(0), // changed: and proved
        append_private_document_entries_operations: Some(0), // changed: a PrivateDocumentCreateFromShieldedPool appends entries to a store
        draw_random_document_properties: Some(0), // changed: sys.random properties are drawn
        delete: DriveDocumentDeleteMethodVersions {
            erase_refund_recipient_cost: Some(0), // changed: prices third-party balance work caused by erasure
            add_estimation_costs_for_remove_document_to_primary_storage: 1, // changed: estimates removal of the composite current pointer
            remove_document_from_primary_storage: 1, // changed: removes only the current history pointer
            delete_document_for_contract_operations: 1, // changed: retains history and records deletion
            erase_document_for_contract_operations: Some(0), // changed: erases one bounded chunk of retained revisions
            add_estimation_costs_for_erase_document: Some(0), // changed: describes the retained revision layers for erasure
            add_estimation_costs_for_lifecycle_record: Some(0), // changed: describes lifecycle metadata layers
            remove_reference_for_index_level_for_contract_operations: 2, // changed: the entries layer is described as references, with sum items under a summable index
            ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4.delete
        },
        estimation_costs: DriveDocumentEstimationCostsMethodVersions {
            add_estimation_costs_for_add_document_to_primary_storage: 1,
            stateless_delete_of_non_tree_for_costs: 1, // changed: each height gets the layer priced there
            ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4.estimation_costs
        },
        update: DriveDocumentUpdateMethodVersions {
            update_document_for_contract_operations: 2, // changed: unchanged index references follow a new owner
            update_document_for_contract_apply_and_add_to_operations: 1, // changed: applied with the batch's owner handover
            ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4.update
        },
        insert_contested: DriveDocumentInsertContestedMethodVersions {
            add_contested_document_for_contract_operations: 2, // changed: end date entries name the storage owner
            ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4.insert_contested
        },
        ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4
    };
