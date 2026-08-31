use crate::version::drive_versions::drive_document_method_versions::{
    DriveDocumentDeleteMethodVersions, DriveDocumentEstimationCostsMethodVersions,
    DriveDocumentIndexUniquenessMethodVersions, DriveDocumentInsertContestedMethodVersions,
    DriveDocumentInsertMethodVersions, DriveDocumentMethodVersions,
    DriveDocumentQueryMethodVersions, DriveDocumentUpdateMethodVersions,
};

/// V4 is protocol version 14's document-method table. It hosts three
/// independent changes that all gate at v14 (ranked aggregates, the
/// shared-prefix aggregate index fix, and the reworked non-primary-key
/// query lowering via `query.non_primary_key_path_query: 1` — multiple
/// `In` clauses, sibling-branch-correct cursor pagination over
/// multi-branch levels, and order-by-aware left-over directions; v13
/// and earlier keep the v0 lowering, which rejects more than one `In`
/// clause and bakes the cursor's start keys into every sibling branch).
///
/// ## 1. Contract-level ranked aggregates
///
/// `query.detect_ranked_mode` is the routing slot for ranked
/// (`HAVING ... TOP/BOTTOM`) aggregate queries served from an indexed
/// property-name tree. The slot exists in every version table so older
/// protocol versions have a value; it is 0 here, and the ranked routing
/// itself is unreachable before the meta-schema-v3 ranked contract
/// grammar activates (which is also v14-gated, via
/// `CONTRACT_VERSIONS_V6`). Pre-v14 protocol versions therefore cannot
/// hold a ranked contract at all.
///
/// ## 2. The shared-prefix aggregate index fix
///
/// V4 differs from V3 in four method-version bumps that fix the
/// shared-prefix aggregate index defect: a contract declaring an
/// aggregating (countable / summable) index terminating at a property
/// that is also the prefix of a compound index (e.g. summable `[a]`
/// next to `[a, b]`) registered fine but rejected every document
/// insert for most flag combinations, because the continuation
/// property-name tree could not be legally hung under the aggregating
/// value tree.
///
/// - `insert.add_indices_for_index_level_for_contract_operations: 1 → 2`
/// - `insert.add_indices_for_top_index_level_for_contract_operations: 1 → 2`
/// - `delete.remove_indices_for_index_level_for_contract_operations: 1 → 2`
/// - `delete.remove_indices_for_top_index_level_for_contract_operations: 1 → 2`
///
/// The v2 walkers derive tree types through the shared
/// continuation-demotion helper (provable count-bearing value trees
/// with compound continuations demote to `CountSumTree`, since grovedb
/// rejects count-suppressed children under provable count parents by
/// design) and route continuation inserts through the completed
/// zero-contribution wrapper matrix. No migration is needed: shapes
/// without compound continuations produce bit-identical operations,
/// the broken shapes could never hold documents, and the one
/// previously-insertable shape the demotion changes (a provable
/// count-bearing value tree whose continuations were all sum-bearing —
/// insertable pre-v14 only through an unenforced grovedb batch guard)
/// simply gets `CountSumTree` value trees for values first seen at
/// v14+, which readers treat identically. Insert and delete bump
/// together because the delete walkers' estimation layer info must
/// describe the exact on-disk shape the insert walkers write.
///
/// v1 walkers stay consensus-locked for protocol v12/v13.
///
/// The two changes compose in the v2 walkers: the ranked upgrade decides
/// the *property-name* tree type (plain → indexed mirror), the
/// continuation demotion decides the *value* tree type, and the two
/// levels never contend. See
/// `packages/rs-drive/src/drive/document/index_level_tree_types.rs`.
pub const DRIVE_DOCUMENT_METHOD_VERSIONS_V4: DriveDocumentMethodVersions =
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
            non_primary_key_path_query: 1,
            non_primary_key_single_in_path_query: 0,
            non_primary_key_multiple_in_path_query: 0,
            where_clause_grouping: 1,
        },
        delete: DriveDocumentDeleteMethodVersions {
            add_estimation_costs_for_remove_document_to_primary_storage: 0,
            delete_document_for_contract: 0,
            delete_document_for_contract_id: 0,
            delete_document_for_contract_apply_and_add_to_operations: 0,
            remove_document_from_primary_storage: 0,
            // v1 at protocol v14: the empty-tree pruning climb stops at the
            // member level on `preallocated` indexOnly indexes, keeping the
            // trees the referenced document's insert paid for.
            remove_reference_for_index_level_for_contract_operations: 1,
            remove_indices_for_index_level_for_contract_operations: 2,
            remove_indices_for_top_index_level_for_contract_operations: 2,
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
            // v1 at protocol v14: inserting a document also preallocates the
            // dynamic trees of `preallocated` indexOnly indexes bound to its
            // type through refersTo declarations. Insert and delete bump
            // together: the delete-side no-prune rule is what makes the
            // preallocated trees permanent structure.
            add_document_for_contract_operations: 1,
            add_document_to_primary_storage: 0,
            add_indices_for_index_level_for_contract_operations: 2,
            add_indices_for_top_index_level_for_contract_operations: 2,
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
            // Bumped alongside the four walkers: a key-changing update
            // materializes index branches itself, so it must derive the
            // same post-demotion tree types and zero-contribution
            // wrappers as the v2 insert walkers or the shapes (and the
            // per-value aggregates) diverge.
            update_document_for_contract_operations: 1,
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
        // Unchanged from V3 — see V3's comment for the v12-gated
        // count/sum composition rationale.
        primary_key_tree_type: 1,
    };
