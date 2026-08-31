use versioned_feature_core::FeatureVersion;

pub mod v1;
pub mod v2;
pub mod v3;
pub mod v4;

#[derive(Clone, Debug, Default)]
pub struct DriveDocumentMethodVersions {
    pub query: DriveDocumentQueryMethodVersions,
    pub delete: DriveDocumentDeleteMethodVersions,
    pub insert: DriveDocumentInsertMethodVersions,
    pub insert_contested: DriveDocumentInsertContestedMethodVersions,
    pub update: DriveDocumentUpdateMethodVersions,
    pub estimation_costs: DriveDocumentEstimationCostsMethodVersions,
    pub index_uniqueness: DriveDocumentIndexUniquenessMethodVersions,
    pub primary_key_tree_type: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveDocumentQueryMethodVersions {
    pub query_documents: FeatureVersion,
    pub query_contested_documents: FeatureVersion,
    pub query_contested_documents_vote_state: FeatureVersion,
    pub query_documents_with_flags: FeatureVersion,
    pub fetch_document_history_query: FeatureVersion,
    pub fetch_document_history: FeatureVersion,
    pub prove_document_history: FeatureVersion,
    /// Mode-detection routing table for `SELECT COUNT` queries.
    /// Versioned because the routing table is consensus-relevant on
    /// the query surface — a future protocol version that changes
    /// shape mappings must land behind a method-version bump.
    pub detect_count_mode: FeatureVersion,
    /// Mode-detection routing table for `SELECT SUM` queries. Same
    /// versioning rationale as `detect_count_mode`.
    pub detect_sum_mode: FeatureVersion,
    /// Mode-detection routing table for ranked (`HAVING ... TOP/BOTTOM/
    /// MIN/MAX`) aggregate queries served from an indexed index tree.
    /// Same versioning rationale as `detect_count_mode`. Present in
    /// every version table so the slot exists for older protocol
    /// versions; the routing itself is unreachable before the ranked
    /// contract grammar activates.
    pub detect_ranked_mode: FeatureVersion,
    /// Mode-detection routing table for boolean `HAVING` range queries
    /// (`GROUP BY p HAVING <agg> <op> <value> LIMIT n`) served from an
    /// indexed tree's axis secondary. Same versioning rationale and
    /// same dormancy pattern as `detect_ranked_mode`: the slot exists
    /// in every table, and the routing is unreachable before both the
    /// ranked contract grammar and the v2 aggregate-routing helper
    /// activate (protocol v14).
    pub detect_having_mode: FeatureVersion,
    /// Lowering of a `DriveDocumentQuery` over a secondary index into a
    /// grovedb `PathQuery`. Versioned because the set of accepted query
    /// shapes is part of the consensus query contract: v0 rejects more
    /// than one non-primary-key `In` clause per query; v1 (protocol
    /// version 14) accepts multiple `In` clauses on consecutive index
    /// properties, lowering them to multi-level key-set path queries.
    pub non_primary_key_path_query: FeatureVersion,
    /// Lowering for query shapes with at most one non-primary-key `In`
    /// clause. Same consensus rationale as
    /// `non_primary_key_path_query`, which routes those shapes here
    /// from its v1 on. Present in every version table so the slot
    /// exists for older protocol versions; unreachable while
    /// `non_primary_key_path_query` is 0, whose v0 lowering carries its
    /// own frozen single-`In` construction.
    pub non_primary_key_single_in_path_query: FeatureVersion,
    /// Lowering for query shapes with multiple non-primary-key `In`
    /// clauses on consecutive index properties, producing multi-level
    /// key-set path queries. Same consensus rationale and reachability
    /// as `non_primary_key_single_in_path_query`.
    pub non_primary_key_multiple_in_path_query: FeatureVersion,
    /// Grouping of a query's raw where clauses into equality / range /
    /// in buckets (`WhereClause::group_clauses`). Versioned because the
    /// error surface for rejected shapes is part of the query contract:
    /// v0 rejects any query with more than one non-primary-key `In`
    /// clause with `MultipleInClauses` before any other same-field
    /// checks; v1 (protocol version 14) groups multiple `In` clauses
    /// structurally and reports duplicate-field and overlap errors
    /// instead. Flips together with `non_primary_key_path_query`.
    pub where_clause_grouping: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveDocumentEstimationCostsMethodVersions {
    pub add_estimation_costs_for_add_document_to_primary_storage: FeatureVersion,
    pub add_estimation_costs_for_add_contested_document_to_primary_storage: FeatureVersion,
    pub stateless_delete_of_non_tree_for_costs: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveDocumentInsertMethodVersions {
    pub add_document: FeatureVersion,
    pub add_history_operations: FeatureVersion,
    pub add_document_for_contract: FeatureVersion,
    pub add_document_for_contract_apply_and_add_to_operations: FeatureVersion,
    pub add_document_for_contract_operations: FeatureVersion,
    pub add_document_to_primary_storage: FeatureVersion,
    pub add_indices_for_index_level_for_contract_operations: FeatureVersion,
    pub add_indices_for_top_index_level_for_contract_operations: FeatureVersion,
    pub add_reference_for_index_level_for_contract_operations: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveDocumentInsertContestedMethodVersions {
    pub add_contested_document: FeatureVersion,
    pub add_contested_document_for_contract: FeatureVersion,
    pub add_contested_document_for_contract_apply_and_add_to_operations: FeatureVersion,
    pub add_contested_document_for_contract_operations: FeatureVersion,
    pub add_contested_document_to_primary_storage: FeatureVersion,
    pub add_contested_indices_for_contract_operations: FeatureVersion,
    pub add_contested_reference_and_vote_subtree_to_document_operations: FeatureVersion,
    pub add_contested_vote_subtree_for_non_identities_operations: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveDocumentUpdateMethodVersions {
    pub add_update_multiple_documents_operations: FeatureVersion,
    pub update_document_for_contract: FeatureVersion,
    pub update_document_for_contract_apply_and_add_to_operations: FeatureVersion,
    pub update_document_for_contract_id: FeatureVersion,
    pub update_document_for_contract_operations: FeatureVersion,
    pub update_document_with_serialization_for_contract: FeatureVersion,
    pub update_serialized_document_for_contract: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveDocumentDeleteMethodVersions {
    pub add_estimation_costs_for_remove_document_to_primary_storage: FeatureVersion,
    pub delete_document_for_contract: FeatureVersion,
    pub delete_document_for_contract_id: FeatureVersion,
    pub delete_document_for_contract_apply_and_add_to_operations: FeatureVersion,
    pub remove_document_from_primary_storage: FeatureVersion,
    pub remove_reference_for_index_level_for_contract_operations: FeatureVersion,
    pub remove_indices_for_index_level_for_contract_operations: FeatureVersion,
    pub remove_indices_for_top_index_level_for_contract_operations: FeatureVersion,
    pub delete_document_for_contract_id_with_named_type_operations: FeatureVersion,
    pub delete_document_for_contract_with_named_type_operations: FeatureVersion,
    pub delete_document_for_contract_operations: FeatureVersion,
    /// indexOnly delete-by-values (dormant slot: present in every table at 0;
    /// only reachable for indexOnly document types, which cannot exist below
    /// PV14).
    pub delete_index_only_document_for_contract_operations: FeatureVersion,
    /// The fee-applying indexOnly deletion wrapper (dormant slot, 0 in
    /// every table; only reachable for indexOnly document types).
    pub delete_index_only_document_for_contract: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveDocumentIndexUniquenessMethodVersions {
    pub validate_document_create_transition_action_uniqueness: FeatureVersion,
    pub validate_document_replace_transition_action_uniqueness: FeatureVersion,
    pub validate_document_transfer_transition_action_uniqueness: FeatureVersion,
    pub validate_document_purchase_transition_action_uniqueness: FeatureVersion,
    pub validate_document_update_price_transition_action_uniqueness: FeatureVersion,
}

#[cfg(test)]
mod historical_method_table_freeze {
    //! Pins the historical (already-shipped) drive document method
    //! tables to their on-chain method-version selections. These
    //! tables route through `DRIVE_VERSION_V*` → `PlatformVersion::V*`
    //! and are dispatched from contract / document write paths whose
    //! output is byte-committed to the merk root, so any field bump
    //! on these tables would silently break replay / sync on the
    //! corresponding mainnet protocol version.
    //!
    //! Add a row below whenever a new platform version ships a drive
    //! document method table; never edit an existing row.

    use super::v2::DRIVE_DOCUMENT_METHOD_VERSIONS_V2;
    use super::v3::DRIVE_DOCUMENT_METHOD_VERSIONS_V3;

    /// V2 was introduced for protocol v10 and is also selected by
    /// protocol v11 (via `DRIVE_VERSION_V5` / `DRIVE_VERSION_V6`).
    /// Every method version here is committed to chain state by
    /// the two release lines that use this table.
    #[test]
    fn v2_primary_key_tree_type_is_frozen_at_v0_dispatch() {
        // The v0 dispatch arm in
        // `packages/rs-drive/src/drive/document/primary_key_tree_type.rs`
        // is count-only: `range_countable → ProvableCountTree`,
        // `documents_countable → CountTree`, else `NormalTree`. Sum
        // flags are ignored. This is the dispatch every v10/v11
        // block committed to.
        //
        // The v3 sum-tree feature uses the v1 dispatch arm via
        // `DRIVE_DOCUMENT_METHOD_VERSIONS_V3.primary_key_tree_type = 1`
        // (platform v12). Bumping V2's value re-routes v10/v11
        // replay through the v1 arm, which is a consensus-breaking
        // change even when the v1 arm's output happens to be
        // semantically equivalent for the actual contracts on chain.
        assert_eq!(
            DRIVE_DOCUMENT_METHOD_VERSIONS_V2.primary_key_tree_type, 0,
            "DRIVE_DOCUMENT_METHOD_VERSIONS_V2.primary_key_tree_type \
             must stay at 0 — V2 is on-chain for protocol versions \
             10 and 11. See the comment in v2.rs for the freeze \
             rationale."
        );
    }

    /// V3 was introduced for protocol v12 alongside the sum-tree
    /// feature. Pinning this catches an inadvertent revert: V3's
    /// `primary_key_tree_type = 1` is what makes the sum-aware
    /// dispatch arm fire under v12.
    #[test]
    fn v3_primary_key_tree_type_selects_v1_dispatch() {
        assert_eq!(
            DRIVE_DOCUMENT_METHOD_VERSIONS_V3.primary_key_tree_type, 1,
            "DRIVE_DOCUMENT_METHOD_VERSIONS_V3.primary_key_tree_type \
             must be 1 — V3 gates the sum-tree feature's count × sum \
             composition (platform v12). See the comment in v3.rs."
        );
    }
}
