use crate::version::drive_versions::drive_document_method_versions::v4::DRIVE_DOCUMENT_METHOD_VERSIONS_V4;
use crate::version::drive_versions::drive_document_method_versions::{
    DriveDocumentDeleteMethodVersions, DriveDocumentEstimationCostsMethodVersions,
    DriveDocumentInsertContestedMethodVersions, DriveDocumentMethodVersions,
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
/// * `draw_random_document_properties` is `Some(0)` (`None` in every earlier table): the
///   documents a block created get their `sys.random` properties drawn from the block's random
///   beacon.
/// * `delete.remove_reference_for_index_level_for_contract_operations` is 2: a delete's estimate
///   describes an index's entries for a value (its `[0]` layer) as the references they are, as
///   the insert's does, not as subtrees.
/// * `estimation_costs.stateless_delete_of_non_tree_for_costs` is 1: a delete's estimate prices
///   each height of the climb with the layer GroveDB deletes in there, not the one above it.
/// * `update.update_document_for_contract_operations` is 2: when an update names a new owner,
///   the references of the indexes it leaves unchanged are rewritten with that owner.
/// * `update.update_document_for_contract_apply_and_add_to_operations` is 1: an update applied
///   on its own hands storage over as one in a batch of drive operations does.
/// * `insert_contested.add_contested_document_for_contract_operations` is 2: a poll's end date
///   entries name whoever the document's storage flags name.
/// * `query.single_document_path_query` is 1: in a document type that keeps history, the lookup
///   of a single document at a block time stops above the pointer to the latest version, and
///   the lookup of a document that may be contested states its cap itself and is refused at a
///   block time.
pub const DRIVE_DOCUMENT_METHOD_VERSIONS_V5: DriveDocumentMethodVersions =
    DriveDocumentMethodVersions {
        insert_empty_private_document_store_operation: Some(0), // changed: a private document type's store is created with it
        fetch_private_document_entries: Some(0), // changed: a page of a store's entries can be read
        prove_private_document_entries: Some(0), // changed: and proved
        append_private_document_entries_operations: Some(0), // changed: a PrivateDocumentCreateFromShieldedPool appends entries to a store
        draw_random_document_properties: Some(0), // changed: sys.random properties are drawn
        query: DriveDocumentQueryMethodVersions {
            single_document_path_query: 1, // changed: a lookup at a block time stops above the pointer to the latest version
            ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4.query
        },
        delete: DriveDocumentDeleteMethodVersions {
            remove_reference_for_index_level_for_contract_operations: 2, // changed: the entries layer is described as references
            ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4.delete
        },
        estimation_costs: DriveDocumentEstimationCostsMethodVersions {
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
