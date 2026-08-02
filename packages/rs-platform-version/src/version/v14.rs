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
/// 2. **The shared-prefix aggregate index fix** (follow-up): an aggregating
///    countable / summable index whose terminal property also prefixes a
///    compound index registers today but rejects document inserts; the v2
///    document index walkers that fix it gate here as well.
///
/// Until a contract uses the ranked grammar, v14 is behaviorally identical
/// to v13:
///
/// * `CONTRACT_VERSIONS_V6` points `document_type_schema` at the v3 document
///   meta-schema, which hosts the ranked index keywords
///   (`rankedCountable` / `rankedSummable` / `rankedAverageable`). v13 keeps
///   validating against meta-schema v2, where those keys are rejected as
///   unknown properties, so a pre-v14 contract cannot smuggle them in.
/// * `DRIVE_VERSION_V9` carries `DRIVE_DOCUMENT_METHOD_VERSIONS_V4`, adding
///   the `detect_ranked_mode` routing slot, plus the grove-method slots for
///   creating the three indexed tree variants and the verify-method slot for
///   `verify_ranked_top_k_proof`. All are 0 today.
/// * `DRIVE_ABCI_QUERY_VERSIONS_V2` bumps
///   `document_query_helpers.compute_aggregate_mode_and_check_limit` 0 → 1,
///   switching the v1 document-query handler from "reject every non-empty
///   HAVING" to routing a `TOP(n)` / `BOTTOM(n)` ranking right-operand to
///   the ranked executor (`MAX` / `MIN` are refused — tie semantics). v13
///   and earlier keep the v1 table and therefore keep rejecting HAVING, so
///   mixed-version networks agree across the upgrade.
///
/// The wire surface is deliberately unchanged: `GetDocumentsRequestV1`
/// already carries `selects` / `group_by` / `having`; the ranked response
/// is an additive `ResultData.ranked` variant.
pub const PLATFORM_V14: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_14,
    drive: DRIVE_VERSION_V9, // changed: drive document method versions v4 (detect_ranked_mode slot)
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
