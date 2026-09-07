use crate::version::drive_versions::drive_document_method_versions::{
    DriveDocumentDeleteMethodVersions, DriveDocumentEstimationCostsMethodVersions,
    DriveDocumentIndexUniquenessMethodVersions, DriveDocumentInsertContestedMethodVersions,
    DriveDocumentInsertMethodVersions, DriveDocumentMethodVersions,
    DriveDocumentQueryMethodVersions, DriveDocumentUpdateMethodVersions,
};

/// This was introduced in protocol v10 to deal with changes in queries for document uniqueness
/// during validation.
pub const DRIVE_DOCUMENT_METHOD_VERSIONS_V2: DriveDocumentMethodVersions =
    DriveDocumentMethodVersions {
        query: DriveDocumentQueryMethodVersions {
            query_documents: 0,
            query_chained_documents: 0,
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
            remove_indices_for_index_level_for_contract_operations: 0,
            remove_indices_for_top_index_level_for_contract_operations: 0,
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
            add_indices_for_index_level_for_contract_operations: 0,
            add_indices_for_top_index_level_for_contract_operations: 0,
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
            validate_document_create_transition_action_uniqueness: 1, // Changed
            validate_document_replace_transition_action_uniqueness: 1, // Changed
            validate_document_transfer_transition_action_uniqueness: 1, // Changed
            validate_document_purchase_transition_action_uniqueness: 1, // Changed
            validate_document_update_price_transition_action_uniqueness: 1, // Changed
        },
        // FROZEN AT 0 for platform versions 10 and 11. Both protocol
        // versions select this table (`DRIVE_DOCUMENT_METHOD_VERSIONS_V2`)
        // via `DRIVE_VERSION_V5` and `DRIVE_VERSION_V6` respectively
        // — they are already shipped, so the method table they
        // dispatch through is part of the chain's historical record
        // and MUST NOT change.
        //
        // The v0 dispatch arm in
        // `packages/rs-drive/src/drive/document/primary_key_tree_type.rs`
        // is the count-only dispatch that existed before the sum-tree
        // feature. It MUST stay selected on v10/v11 so contract-insert,
        // document-insert, and document-delete replay on those
        // protocol versions reproduces the exact grovedb `TreeType`
        // choices each block originally committed to.
        //
        // The sum-tree feature's count × sum composition lives in the
        // v1 dispatch arm and is selected by
        // `DRIVE_DOCUMENT_METHOD_VERSIONS_V3.primary_key_tree_type = 1`
        // (used by `DRIVE_VERSION_V7` / platform v12, the version
        // that introduces `documentsSummable` / `rangeSummable` at
        // the DPP parser via `try_from_schema: 2`).
        //
        // It is observably true today that pre-v12 contracts can't
        // carry sum flags (DPP v11's `try_from_schema: 1` doesn't
        // read those fields, so `documents_summable.is_none()` and
        // `range_summable == false` for every valid v11 contract),
        // which makes the v1 arm's output semantically equivalent to
        // the v0 arm's for valid v11 history. That equivalence is a
        // happy property — not a license to edit V2. The versioning
        // contract requires V2 to be byte-for-byte frozen, full
        // stop, so a future change to the v1 arm doesn't need to
        // re-prove v0 ≡ v1 for every pre-v12 corner case.
        primary_key_tree_type: 0,
    };
