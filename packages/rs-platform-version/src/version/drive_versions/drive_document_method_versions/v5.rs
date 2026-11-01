use crate::version::drive_versions::drive_document_method_versions::v4::DRIVE_DOCUMENT_METHOD_VERSIONS_V4;
use crate::version::drive_versions::drive_document_method_versions::{
    DriveDocumentDeleteMethodVersions, DriveDocumentEstimationCostsMethodVersions,
    DriveDocumentMethodVersions,
};

/// V5 is protocol version 15's document-method table. Relative to V4:
///
/// * `insert_empty_private_document_store_operation` is `Some(0)` (`None` in every earlier
///   table): a contract registering a private document type, or an update adding one, creates
///   the type's private document store at its key `[0]`, sized from the private document store
///   limits of `SYSTEM_LIMITS_V5`.
/// * `delete.remove_reference_for_index_level_for_contract_operations` is 2: a delete's estimate
///   describes an index's entries for a value (its `[0]` layer) as the references they are, as
///   the insert's does, not as subtrees.
/// * `estimation_costs.stateless_delete_of_non_tree_for_costs` is 1: a delete's estimate prices
///   each height of the climb with the layer GroveDB deletes in there, not the one above it.
pub const DRIVE_DOCUMENT_METHOD_VERSIONS_V5: DriveDocumentMethodVersions =
    DriveDocumentMethodVersions {
        insert_empty_private_document_store_operation: Some(0), // changed: a private document type's store is created with it
        delete: DriveDocumentDeleteMethodVersions {
            remove_reference_for_index_level_for_contract_operations: 2, // changed: the entries layer is described as references
            ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4.delete
        },
        estimation_costs: DriveDocumentEstimationCostsMethodVersions {
            stateless_delete_of_non_tree_for_costs: 1, // changed: each height gets the layer priced there
            ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4.estimation_costs
        },
        ..DRIVE_DOCUMENT_METHOD_VERSIONS_V4
    };
