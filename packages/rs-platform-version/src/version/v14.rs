use crate::version::consensus_versions::ConsensusVersions;
use crate::version::dpp_versions::dpp_asset_lock_versions::v1::DPP_ASSET_LOCK_VERSIONS_V1;
use crate::version::dpp_versions::dpp_contract_versions::v6::CONTRACT_VERSIONS_V6;
use crate::version::dpp_versions::dpp_costs_versions::v1::DPP_COSTS_VERSIONS_V1;
use crate::version::dpp_versions::dpp_document_versions::v3::DOCUMENT_VERSIONS_V3;
use crate::version::dpp_versions::dpp_factory_versions::v1::DPP_FACTORY_VERSIONS_V1;
use crate::version::dpp_versions::dpp_identity_versions::v1::IDENTITY_VERSIONS_V1;
use crate::version::dpp_versions::dpp_method_versions::v2::DPP_METHOD_VERSIONS_V2;
use crate::version::dpp_versions::dpp_state_transition_conversion_versions::v2::STATE_TRANSITION_CONVERSION_VERSIONS_V2;
use crate::version::dpp_versions::dpp_state_transition_method_versions::v1::STATE_TRANSITION_METHOD_VERSIONS_V1;
use crate::version::dpp_versions::dpp_state_transition_serialization_versions::v2::STATE_TRANSITION_SERIALIZATION_VERSIONS_V2;
use crate::version::dpp_versions::dpp_state_transition_versions::v3::STATE_TRANSITION_VERSIONS_V3;
use crate::version::dpp_versions::dpp_token_versions::v2::TOKEN_VERSIONS_V2;
use crate::version::dpp_versions::dpp_validation_versions::v4::DPP_VALIDATION_VERSIONS_V4;
use crate::version::dpp_versions::dpp_voting_versions::v2::VOTING_VERSION_V2;
use crate::version::dpp_versions::DPPVersion;
use crate::version::drive_abci_versions::drive_abci_checkpoint_parameters::v1::DRIVE_ABCI_CHECKPOINT_PARAMETERS_V1;
use crate::version::drive_abci_versions::drive_abci_method_versions::v9::DRIVE_ABCI_METHOD_VERSIONS_V9;
use crate::version::drive_abci_versions::drive_abci_query_versions::v2::DRIVE_ABCI_QUERY_VERSIONS_V2;
use crate::version::drive_abci_versions::drive_abci_structure_versions::v1::DRIVE_ABCI_STRUCTURE_VERSIONS_V1;
use crate::version::drive_abci_versions::drive_abci_validation_versions::v9::DRIVE_ABCI_VALIDATION_VERSIONS_V9;
use crate::version::drive_abci_versions::drive_abci_withdrawal_constants::v2::DRIVE_ABCI_WITHDRAWAL_CONSTANTS_V2;
use crate::version::drive_abci_versions::DriveAbciVersion;
use crate::version::drive_versions::v9::DRIVE_VERSION_V9;
use crate::version::fee::v2::FEE_VERSION2;
use crate::version::protocol_version::PlatformVersion;
use crate::version::system_data_contract_versions::v2::SYSTEM_DATA_CONTRACT_VERSIONS_V2;
use crate::version::system_limits::v3::SYSTEM_LIMITS_V3;
use crate::version::ProtocolVersion;

pub const PROTOCOL_VERSION_14: ProtocolVersion = 14;

/// v14 hosts two consensus changes:
///
/// 1. **Contract-level ranked aggregates** (this branch): an index can
///    declare that its groups are rankable by an aggregate, so a query like
///    "top 5 restaurants by average grade" is served from an ordered
///    secondary tree in O(log n + k) with a proof, instead of being rejected.
/// 2. **The shared-prefix aggregate index fix**: a data contract declaring
///    an aggregating (countable / summable) index that terminates at a
///    property which is also the prefix of a compound index (e.g. summable
///    `[a]` next to `[a, b]`) registered successfully but rejected every
///    document insert for most flag combinations, because Drive could not
///    legally hang the compound continuation tree under the aggregating
///    per-value tree. The v2 document index walkers (plus the v1 update
///    walker) that fix it gate here as well: tree types derive through a
///    shared continuation-demotion helper (provable count-bearing value
///    trees with compound continuations demote to `CountSumTree`, since
///    grovedb rejects count-suppressed children under provable count
///    parents by design) and continuation inserts route through the
///    completed zero-contribution wrapper matrix. No state migration is
///    needed: shapes without compound continuations produce bit-identical
///    operations, the broken shapes could never hold documents, and the
///    one previously-insertable shape the demotion changes (a provable
///    count-bearing value tree whose continuations were all sum-bearing —
///    insertable pre-v14 only through an unenforced grovedb batch guard)
///    simply gets `CountSumTree` value trees for values first seen at
///    v14+, which readers treat identically.
///
/// The two are orthogonal by construction: the ranked upgrade decides the
/// *property-name* tree type, the demotion decides the *value* tree type
/// one level below it, and a demoted `CountSumTree` value tree contributes
/// its (count, sum) to a ranked indexed parent exactly as the provable
/// variant did — so ranked secondaries keep ranking correctly over
/// shared-prefix shapes.
///
/// Until a contract uses the ranked grammar, the only v14 behavior change
/// is the shared-prefix fix; everything else matches v13:
///
/// * `CONTRACT_VERSIONS_V6` points `document_type_schema` at the v3 document
///   meta-schema, which hosts the ranked index keywords
///   (`rankedCountable` / `rankedSummable` / `rankedAverageable`). v13 keeps
///   validating against meta-schema v2, where those keys are rejected as
///   unknown properties, so a pre-v14 contract cannot smuggle them in.
/// * `DRIVE_VERSION_V9` carries `DRIVE_DOCUMENT_METHOD_VERSIONS_V4`, adding
///   the `detect_ranked_mode` routing slot, plus the grove-method slots for
///   creating the three indexed tree variants and the verify-method slot for
///   `verify_ranked_top_k_proof`. All are 0 today. The same table bumps the
///   four index walkers to v2 and the document update walker to v1 for the
///   shared-prefix fix.
/// * `DRIVE_ABCI_QUERY_VERSIONS_V2` bumps
///   `document_query_helpers.compute_aggregate_mode_and_check_limit` 0 → 1,
///   opening the ranked path on the v1 document-query handler: a grouped
///   aggregate whose single `order_by` names the selected aggregate
///   (`ORDER BY <agg> [ASC|DESC] LIMIT n [OFFSET m]`) routes to the ranked
///   executor. v13 and earlier keep the v1 table and therefore keep
///   rejecting that shape, so mixed-version networks agree across the
///   upgrade.
///
/// The wire surface is deliberately unchanged: `GetDocumentsRequestV1`
/// already carries `selects` / `group_by` / `order_by` / `limit` /
/// `offset`; the ranked response is an additive `ResultData.ranked`
/// variant, whose `skipped` field is likewise additive.
pub const PLATFORM_V14: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_14,
    drive: DRIVE_VERSION_V9, // changed: drive document method versions v4 — v2 index walkers (shared-prefix aggregate indexes become insertable) + the detect_ranked_mode slot
    drive_abci: DriveAbciVersion {
        structs: DRIVE_ABCI_STRUCTURE_VERSIONS_V1,
        methods: DRIVE_ABCI_METHOD_VERSIONS_V9,
        validation_and_processing: DRIVE_ABCI_VALIDATION_VERSIONS_V9,
        withdrawal_constants: DRIVE_ABCI_WITHDRAWAL_CONSTANTS_V2,
        query: DRIVE_ABCI_QUERY_VERSIONS_V2, // changed: ranked HAVING routing gate
        checkpoints: DRIVE_ABCI_CHECKPOINT_PARAMETERS_V1,
    },
    dpp: DPPVersion {
        costs: DPP_COSTS_VERSIONS_V1,
        validation: DPP_VALIDATION_VERSIONS_V4,
        state_transition_serialization_versions: STATE_TRANSITION_SERIALIZATION_VERSIONS_V2,
        state_transition_conversion_versions: STATE_TRANSITION_CONVERSION_VERSIONS_V2,
        state_transition_method_versions: STATE_TRANSITION_METHOD_VERSIONS_V1,
        state_transitions: STATE_TRANSITION_VERSIONS_V3,
        contract_versions: CONTRACT_VERSIONS_V6, // changed: v3 document meta-schema hosts the ranked index keywords
        document_versions: DOCUMENT_VERSIONS_V3,
        identity_versions: IDENTITY_VERSIONS_V1,
        voting_versions: VOTING_VERSION_V2,
        token_versions: TOKEN_VERSIONS_V2,
        asset_lock_versions: DPP_ASSET_LOCK_VERSIONS_V1,
        methods: DPP_METHOD_VERSIONS_V2,
        factory_versions: DPP_FACTORY_VERSIONS_V1,
    },
    system_data_contracts: SYSTEM_DATA_CONTRACT_VERSIONS_V2,
    fee_version: FEE_VERSION2,
    system_limits: SYSTEM_LIMITS_V3,
    consensus: ConsensusVersions {
        tenderdash_consensus_version: 1,
    },
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::version::v13::PLATFORM_V13;

    /// The ranked-HAVING routing gate lives in v14's own query table, so
    /// flipping it to feature version 1 touches only v14: a v13 node keeps
    /// running the v0 helper, which rejects every non-empty HAVING, so a
    /// mixed-version network agrees until the upgrade vote carries.
    ///
    /// The flip is real as of the ranked routing landing — v14 selects the
    /// v1 helper, which routes a single ranking-operand HAVING clause to
    /// `dispatch_ranked_v1`. A change that made v13 non-zero here would be
    /// consensus-breaking for already-deployed nodes, which is exactly what
    /// the v13 half of this assertion guards.
    #[test]
    fn ranked_having_routing_gate_is_v14_only() {
        assert_eq!(
            PLATFORM_V13
                .drive_abci
                .query
                .document_query_helpers
                .compute_aggregate_mode_and_check_limit,
            0
        );
        assert_eq!(
            PLATFORM_V14
                .drive_abci
                .query
                .document_query_helpers
                .compute_aggregate_mode_and_check_limit,
            1
        );
    }

    /// The ranked index keywords are gated by the meta-schema version, so v14
    /// must select meta-schema v3 while v13 stays on v2.
    #[test]
    fn ranked_index_keywords_are_gated_by_meta_schema_v3() {
        assert_eq!(
            PLATFORM_V13
                .dpp
                .contract_versions
                .document_type_versions
                .schema
                .document_type_schema,
            2
        );
        assert_eq!(
            PLATFORM_V14
                .dpp
                .contract_versions
                .document_type_versions
                .schema
                .document_type_schema,
            3
        );
    }

    /// The ranked grammar lives in its own document-type parser generation
    /// rather than behind a version gate inside a shipped one, so v14 must
    /// select generation 3 while v13 stays on generation 2. Pinned here
    /// because it is the whole reason generations 0/1/2 can stay byte-identical
    /// to what consensus already ran: a historical block replayed at v13 is
    /// parsed by a generation that has never heard of the ranked keywords.
    /// The grove v4 cleanup gates (batch overwrite inspection + delete-tree
    /// actual-type cleanup) exist for the indexed trees that ranked indexes
    /// lay down, so v14 must select grove protocol 4 while v13 stays on 3.
    /// The gates are cost-neutral — they derive the old element from data the
    /// merk apply already loads — and the fee-constant tests pin identical
    /// fees on both sides of the boundary. Platform flows cannot themselves
    /// overwrite a ranked index (the flags are immutable on contract update
    /// and new indexes cannot be added to an existing document type), so the
    /// cleanup behavior itself is exercised by grovedb's own overwrite suites
    /// at the pinned revision; this test pins that v14 actually activates
    /// them.
    #[test]
    fn grove_v4_cleanup_gates_activate_at_v14() {
        assert_eq!(PLATFORM_V13.drive.grove_version.protocol_version, 3);
        assert_eq!(PLATFORM_V14.drive.grove_version.protocol_version, 4);
    }

    #[test]
    fn ranked_grammar_gets_its_own_parser_generation() {
        assert_eq!(
            PLATFORM_V13
                .dpp
                .contract_versions
                .document_type_versions
                .class_method_versions
                .try_from_schema,
            2
        );
        assert_eq!(
            PLATFORM_V14
                .dpp
                .contract_versions
                .document_type_versions
                .class_method_versions
                .try_from_schema,
            3
        );
    }

    /// v14 introduces the slots but activates none of them yet. If a later
    /// change flips one of these, it must do so deliberately — and update this
    /// test — rather than by inheriting a default.
    #[test]
    fn ranked_feature_slots_exist_but_are_dormant() {
        assert_eq!(
            PLATFORM_V14.drive.methods.document.query.detect_ranked_mode,
            0
        );
        assert_eq!(
            PLATFORM_V14
                .drive
                .methods
                .verify
                .document_ranked
                .verify_ranked_top_k_proof,
            0
        );
        let grove = &PLATFORM_V14.drive.grove_methods.batch;
        assert_eq!(grove.batch_insert_empty_provable_count_indexed_tree, 0);
        assert_eq!(grove.batch_insert_empty_provable_sum_indexed_tree, 0);
        assert_eq!(
            grove.batch_insert_empty_provable_count_provable_sum_indexed_tree,
            0
        );
    }
}
