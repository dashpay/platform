use crate::version::drive_versions::drive_document_method_versions::{
    DriveDocumentDeleteMethodVersions, DriveDocumentEstimationCostsMethodVersions,
    DriveDocumentIndexUniquenessMethodVersions, DriveDocumentInsertContestedMethodVersions,
    DriveDocumentInsertMethodVersions, DriveDocumentMethodVersions,
    DriveDocumentQueryMethodVersions, DriveDocumentUpdateMethodVersions,
};

/// V3 differs from V2 in three method-version bumps that switch the
/// index-walker estimation paths from `EstimatedLayerSizes::AllSubtrees(.., NoSumTrees, ..)`
/// to the sum-aware shortcut that maps the actual value tree type
/// onto `SomeSumTrees`'s matching weight slot (grovedb #674):
///
/// - `insert.add_indices_for_index_level_for_contract_operations: 0 → 1`
/// - `insert.add_indices_for_top_index_level_for_contract_operations: 0 → 1`
/// - `delete.remove_indices_for_index_level_for_contract_operations: 0 → 1`
///
/// v0 paths remain available for pre-v12 platform versions (consensus
/// baseline locked); v1 only becomes active when this V3 table is
/// selected, i.e. at protocol v12+ via `DRIVE_VERSION_V7`.
pub const DRIVE_DOCUMENT_METHOD_VERSIONS_V3: DriveDocumentMethodVersions =
    DriveDocumentMethodVersions {
        query: DriveDocumentQueryMethodVersions {
            query_documents: 0,
            query_contested_documents: 0,
            query_contested_documents_vote_state: 0,
            query_documents_with_flags: 0,
            fetch_document_history_query: 0,
            fetch_document_history: 0,
            prove_document_history: 0,
            detect_count_mode: 0,
            detect_sum_mode: 0,
            detect_ranked_mode: 0,
            detect_having_mode: 0,
            non_primary_key_path_query: 0,
            non_primary_key_single_in_path_query: 0,
            non_primary_key_multiple_in_path_query: 0,
            where_clause_grouping: 0,
        },
        delete: DriveDocumentDeleteMethodVersions {
            add_estimation_costs_for_remove_document_to_primary_storage: 0,
            delete_document_for_contract: 0,
            delete_document_for_contract_id: 0,
            delete_document_for_contract_apply_and_add_to_operations: 0,
            remove_document_from_primary_storage: 0,
            remove_reference_for_index_level_for_contract_operations: 0,
            // Bumped: v1 derives value_tree_type from the four-axis
            // flags and feeds it into `estimated_sum_trees_for_value_tree_type`,
            // so the property-name layer's estimation accounts for
            // per-node aggregate bytes on summable / range_summable
            // / count-sum / PCPS value trees.
            remove_indices_for_index_level_for_contract_operations: 1,
            remove_indices_for_top_index_level_for_contract_operations: 1,
            delete_document_for_contract_id_with_named_type_operations: 0,
            delete_document_for_contract_with_named_type_operations: 0,
            delete_document_for_contract_operations: 0,
            delete_index_only_document_for_contract_operations: 0,
            delete_index_only_document_for_contract: 0,
        },
        insert: DriveDocumentInsertMethodVersions {
            add_document: 0,
            add_history_operations: 0,
            add_document_for_contract: 0,
            add_document_for_contract_apply_and_add_to_operations: 0,
            add_document_for_contract_operations: 0,
            add_document_to_primary_storage: 0,
            // Both insert walkers bumped to v1 for the same fix:
            // property-name layer's children are value trees of the
            // computed `value_tree_type`, so use the sum-aware shortcut
            // instead of `NoSumTrees`.
            add_indices_for_index_level_for_contract_operations: 1,
            add_indices_for_top_index_level_for_contract_operations: 1,
            add_reference_for_index_level_for_contract_operations: 0,
        },
        insert_contested: DriveDocumentInsertContestedMethodVersions {
            add_contested_document: 0,
            add_contested_document_for_contract: 0,
            add_contested_document_for_contract_apply_and_add_to_operations: 0,
            add_contested_document_for_contract_operations: 0,
            add_contested_document_to_primary_storage: 0,
            add_contested_indices_for_contract_operations: 0,
            add_contested_reference_and_vote_subtree_to_document_operations: 0,
            add_contested_vote_subtree_for_non_identities_operations: 0,
        },
        update: DriveDocumentUpdateMethodVersions {
            add_update_multiple_documents_operations: 0,
            update_document_for_contract: 0,
            update_document_for_contract_apply_and_add_to_operations: 0,
            update_document_for_contract_id: 0,
            update_document_for_contract_operations: 0,
            update_document_with_serialization_for_contract: 0,
            update_serialized_document_for_contract: 0,
        },
        estimation_costs: DriveDocumentEstimationCostsMethodVersions {
            add_estimation_costs_for_add_document_to_primary_storage: 0,
            add_estimation_costs_for_add_contested_document_to_primary_storage: 0,
            stateless_delete_of_non_tree_for_costs: 0,
        },
        index_uniqueness: DriveDocumentIndexUniquenessMethodVersions {
            validate_document_create_transition_action_uniqueness: 1,
            validate_document_replace_transition_action_uniqueness: 1,
            validate_document_transfer_transition_action_uniqueness: 1,
            validate_document_purchase_transition_action_uniqueness: 1,
            validate_document_update_price_transition_action_uniqueness: 1,
        },
        // Bumped to 1 vs V2's frozen 0: this is the v12-gated entry
        // point for the sum-tree feature. The v1 dispatch arm in
        // `packages/rs-drive/src/drive/document/primary_key_tree_type.rs`
        // composes count + sum flags from `DocumentTypeV2::documents_countable`
        // / `documents_summable` (+ their `range_*` siblings) into the
        // right grovedb `TreeType` — including the combined
        // `CountSumTree` / `ProvableCountSumTree` /
        // `ProvableCountProvableSumTree` variants. Pre-v12 protocol
        // versions stay on V2's v0 dispatch via their own method
        // tables (see V2's comment for the freeze rationale).
        primary_key_tree_type: 1,
    };
