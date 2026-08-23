use crate::version::consensus_versions::ConsensusVersions;
use crate::version::dpp_versions::dpp_asset_lock_versions::v1::DPP_ASSET_LOCK_VERSIONS_V1;
use crate::version::dpp_versions::dpp_contract_versions::v6::CONTRACT_VERSIONS_V6;
use crate::version::dpp_versions::dpp_costs_versions::v1::DPP_COSTS_VERSIONS_V1;
use crate::version::dpp_versions::dpp_document_versions::v3::DOCUMENT_VERSIONS_V3;
use crate::version::dpp_versions::dpp_factory_versions::v1::DPP_FACTORY_VERSIONS_V1;
use crate::version::dpp_versions::dpp_identity_versions::v1::IDENTITY_VERSIONS_V1;
use crate::version::dpp_versions::dpp_method_versions::v3::DPP_METHOD_VERSIONS_V3;
use crate::version::dpp_versions::dpp_state_transition_conversion_versions::v2::STATE_TRANSITION_CONVERSION_VERSIONS_V2;
use crate::version::dpp_versions::dpp_state_transition_method_versions::v1::STATE_TRANSITION_METHOD_VERSIONS_V1;
use crate::version::dpp_versions::dpp_state_transition_serialization_versions::v2::STATE_TRANSITION_SERIALIZATION_VERSIONS_V2;
use crate::version::dpp_versions::dpp_state_transition_versions::v3::STATE_TRANSITION_VERSIONS_V3;
use crate::version::dpp_versions::dpp_token_versions::v2::TOKEN_VERSIONS_V2;
use crate::version::dpp_versions::dpp_validation_versions::v5::DPP_VALIDATION_VERSIONS_V5;
use crate::version::dpp_versions::dpp_voting_versions::v2::VOTING_VERSION_V2;
use crate::version::dpp_versions::DPPVersion;
use crate::version::drive_abci_versions::drive_abci_checkpoint_parameters::v1::DRIVE_ABCI_CHECKPOINT_PARAMETERS_V1;
use crate::version::drive_abci_versions::drive_abci_method_versions::v10::DRIVE_ABCI_METHOD_VERSIONS_V10;
use crate::version::drive_abci_versions::drive_abci_query_versions::v3::DRIVE_ABCI_QUERY_VERSIONS_V3;
use crate::version::drive_abci_versions::drive_abci_structure_versions::v1::DRIVE_ABCI_STRUCTURE_VERSIONS_V1;
use crate::version::drive_abci_versions::drive_abci_validation_versions::v10::DRIVE_ABCI_VALIDATION_VERSIONS_V10;
use crate::version::drive_abci_versions::drive_abci_withdrawal_constants::v3::DRIVE_ABCI_WITHDRAWAL_CONSTANTS_V3;
use crate::version::drive_abci_versions::DriveAbciVersion;
use crate::version::drive_versions::v9::DRIVE_VERSION_V9;
use crate::version::fee::v2::FEE_VERSION2;
use crate::version::protocol_version::PlatformVersion;
use crate::version::system_data_contract_versions::v3::SYSTEM_DATA_CONTRACT_VERSIONS_V3;
use crate::version::system_limits::v4::SYSTEM_LIMITS_V4;
use crate::version::ProtocolVersion;

pub const PROTOCOL_VERSION_14: ProtocolVersion = 14;

/// v14 hosts four consensus changes:
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
/// 3. **The contested vote poll index cross-check**: the index named by a
///    document create transition's prefunded voting balance keys the vote
///    poll, its stored info, its end-date entry and its prefunded
///    specialized balance, while the contested index the contender is
///    inserted under always comes from the document type. Up to v13 nothing
///    tied the two together, so a submitter could register and fund a
///    contest under a vote poll describing a different index than the one
///    the contest was created on — which halts the chain when that poll
///    ends — or open a contest for a document that is not a contested
///    resource at all.
/// 4. **Relative daily withdrawal limit**: the flat 2000 Dash per 24 hours that
///    applied from v8 becomes 15% of the total credits Platform held a day ago
///    (`SYSTEM_LIMITS_V4.daily_withdrawal_limit_percent`, read by
///    `daily_withdrawal_limit` v2 through `DPP_METHOD_VERSIONS_V3`), never below
///    one maximal withdrawal (`max_withdrawal_amount`) so every accepted
///    withdrawal eventually fits and cannot block the pooling queue, and never
///    above `max_daily_withdrawal_amount` (4000 Dash, Core's unlock capacity per
///    day under V24) since pooling more than Core mines only cycles through
///    expiry and re-signing. The base is
///    the total credits recorded at the latest block at least 24 hours before
///    the current one: `DRIVE_ABCI_METHOD_VERSIONS_V10` turns on
///    `record_total_credits_history_for_withdrawals`, which checks the total
///    credits every block once fees and epoch rewards are in, writes it under
///    the withdrawals tree keyed by block time whenever it changed (an entry
///    describes the total until the next one) and prunes entries older than the
///    one the limit reads, and `DRIVE_VERSION_V9`'s identity withdrawal table
///    bumps `calculate_current_withdrawal_limit` to 1 to read that lagged
///    value. Until an entry is a day old — the first day after activation — the
///    flat 2000 Dash keeps applying, so the lag cannot be skipped by inflating
///    the total before or at activation. The lag is the guardrail: a sudden
///    jump in the total credits does not raise the limit for a day. Amounts
///    already pooled in the last 24 hours keep counting against the maximum
///    exactly as before. Core's own unlock limit is unaffected: pre-V24 Core
///    caps unlocks at `LimitAmountV22` (2000 Dash) per *block*, with the amount
///    checked only at block level, so any daily total is still minable across
///    blocks; after V24 it enforces 4000 Dash per 576-block window, which the
///    cap above never exceeds.
///
/// The first two are orthogonal by construction: the ranked upgrade decides the
/// *property-name* tree type, the demotion decides the *value* tree type
/// one level below it, and a demoted `CountSumTree` value tree contributes
/// its (count, sum) to a ranked indexed parent exactly as the provable
/// variant did — so ranked secondaries keep ranking correctly over
/// shared-prefix shapes.
///
/// Until a contract uses the ranked grammar, the only v14 behavior changes
/// are the shared-prefix fix, the contested-index cross-check, the
/// index-reorder schema-compatibility fix and the relative daily withdrawal
/// limit; everything else matches v13:
///
/// * `CONTRACT_VERSIONS_V6` points `document_type_schema` at the v3 document
///   meta-schema, which hosts the ranked index keywords
///   (`rankedCountable` / `rankedSummable` / `rankedAverageable`). v13 keeps
///   validating against meta-schema v2, where those keys are rejected as
///   unknown properties, so a pre-v14 contract cannot smuggle them in.
///   It also bumps `validate_schema_compatibility` to 1, which strips the
///   top-level `indices` key before diffing the old and new document type
///   schemas: index immutability is enforced by `validate_update` v1's
///   name-keyed comparison, so a contract update that merely reorders the
///   `indices` array validates cleanly instead of hitting the
///   unsupported-keyword hard error (an internal error under v13).
/// * `DRIVE_VERSION_V9` carries `DRIVE_DOCUMENT_METHOD_VERSIONS_V4`, adding
///   the `detect_ranked_mode` routing slot, plus the grove-method slots for
///   creating the three indexed tree variants and the verify-method slot for
///   `verify_ranked_top_k_proof`. All are 0 today. The same table bumps the
///   four index walkers to v2 and the document update walker to v1 for the
///   shared-prefix fix.
/// * `DRIVE_ABCI_QUERY_VERSIONS_V3` bumps
///   `document_query_helpers.compute_aggregate_mode_and_check_limit` 0 → 2,
///   opening two routes on the v1 document-query handler: the ranked path
///   (a grouped aggregate whose single `order_by` names the selected
///   aggregate — `ORDER BY <agg> [ASC|DESC] LIMIT n [OFFSET m]`) and the
///   boolean-`HAVING` range path (a grouped aggregate carrying exactly one
///   `having` clause on the selected aggregate — `GROUP BY p HAVING <agg>
///   <op> <value> LIMIT n`), the latter served as a value-bounded range
///   read of the covering ranked index's axis secondary. v13 and earlier
///   keep the v1 table and therefore keep rejecting both shapes, so
///   mixed-version networks agree across the upgrade.
/// * `DRIVE_ABCI_VALIDATION_VERSIONS_V10` bumps
///   `document_create_transition_structure_validation` 0 → 1, requiring a
///   contested create transition's prefunded voting balance to name the
///   same vote poll the document itself resolves to, and rejecting one on a
///   document that resolves to no contested index. It also bumps document
///   create state validation to 2 and document replace state validation to
///   1, enforcing `refersTo` document references: a document whose
///   reference property names an identity or contract that does not exist
///   is rejected. v13 keeps the v9 table and therefore keeps
///   accepting all of these, so replay of pre-upgrade blocks is unchanged.
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
        methods: DRIVE_ABCI_METHOD_VERSIONS_V10, // changed: records the per-block total credits history for the daily withdrawal limit
        validation_and_processing: DRIVE_ABCI_VALIDATION_VERSIONS_V10, // changed: contested-index cross-check + refersTo document reference validation
        withdrawal_constants: DRIVE_ABCI_WITHDRAWAL_CONSTANTS_V3, // changed: prune bound for the total credits history
        query: DRIVE_ABCI_QUERY_VERSIONS_V3, // changed: ranked + boolean-HAVING routing gate
        checkpoints: DRIVE_ABCI_CHECKPOINT_PARAMETERS_V1,
    },
    dpp: DPPVersion {
        costs: DPP_COSTS_VERSIONS_V1,
        validation: DPP_VALIDATION_VERSIONS_V5,
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
        methods: DPP_METHOD_VERSIONS_V3, // changed: daily_withdrawal_limit v2 — a percentage of the total credits a day ago
        factory_versions: DPP_FACTORY_VERSIONS_V1,
    },
    system_data_contracts: SYSTEM_DATA_CONTRACT_VERSIONS_V3, // changed: DashPay v2 adds profile payment address fields (DIP-33)
    fee_version: FEE_VERSION2,
    system_limits: SYSTEM_LIMITS_V4, // changed: daily withdrawal limit becomes 15% of the total credits a day ago
    consensus: ConsensusVersions {
        tenderdash_consensus_version: 1,
    },
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::version::v13::PLATFORM_V13;

    /// The ranked / boolean-HAVING routing gate lives in v14's own query
    /// table, so flipping it touches only v14: a v13 node keeps running
    /// the v0 helper, which rejects every non-empty HAVING, so a
    /// mixed-version network agrees until the upgrade vote carries.
    ///
    /// v14 selects the v2 helper, which routes the ranked shape
    /// (`ORDER BY <agg> LIMIT n`) to `dispatch_ranked_v1` and the
    /// boolean-HAVING range shape (exactly one `having` clause on the
    /// selected aggregate) to `dispatch_having_v1`. A change that made
    /// v13 non-zero here would be consensus-breaking for
    /// already-deployed nodes, which is exactly what the v13 half of
    /// this assertion guards.
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
            2
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
            PLATFORM_V14.drive.methods.document.query.detect_having_mode,
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
        assert_eq!(
            PLATFORM_V14
                .drive
                .methods
                .verify
                .document_ranked
                .verify_having_range_proof,
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

    /// The contested vote poll index cross-check changes accept/reject
    /// behavior for document create transitions, so it lives in v14's own
    /// validation table: a v13 node keeps running structure validation v0,
    /// which validates only the prefunded amount and ignores the index name.
    /// A change that made v13 non-zero here would retroactively reject
    /// transitions already in the chain.
    #[test]
    fn contested_index_cross_check_is_v14_only() {
        assert_eq!(
            PLATFORM_V13
                .drive_abci
                .validation_and_processing
                .state_transitions
                .batch_state_transition
                .document_create_transition_structure_validation,
            0
        );
        assert_eq!(
            PLATFORM_V14
                .drive_abci
                .validation_and_processing
                .state_transitions
                .batch_state_transition
                .document_create_transition_structure_validation,
            1
        );
    }
}
