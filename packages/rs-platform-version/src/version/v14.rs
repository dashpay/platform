use crate::version::consensus_versions::ConsensusVersions;
use crate::version::dpp_versions::dpp_asset_lock_versions::v1::DPP_ASSET_LOCK_VERSIONS_V1;
use crate::version::dpp_versions::dpp_contract_versions::v5::CONTRACT_VERSIONS_V5;
use crate::version::dpp_versions::dpp_costs_versions::v1::DPP_COSTS_VERSIONS_V1;
use crate::version::dpp_versions::dpp_document_versions::v3::DOCUMENT_VERSIONS_V3;
use crate::version::dpp_versions::dpp_factory_versions::v1::DPP_FACTORY_VERSIONS_V1;
use crate::version::dpp_versions::dpp_identity_versions::v1::IDENTITY_VERSIONS_V1;
use crate::version::dpp_versions::dpp_method_versions::v2::DPP_METHOD_VERSIONS_V2;
use crate::version::dpp_versions::dpp_state_transition_conversion_versions::v2::STATE_TRANSITION_CONVERSION_VERSIONS_V2;
use crate::version::dpp_versions::dpp_state_transition_method_versions::v2::STATE_TRANSITION_METHOD_VERSIONS_V2;
use crate::version::dpp_versions::dpp_state_transition_serialization_versions::v2::STATE_TRANSITION_SERIALIZATION_VERSIONS_V2;
use crate::version::dpp_versions::dpp_state_transition_versions::v3::STATE_TRANSITION_VERSIONS_V3;
use crate::version::dpp_versions::dpp_token_versions::v2::TOKEN_VERSIONS_V2;
use crate::version::dpp_versions::dpp_validation_versions::v4::DPP_VALIDATION_VERSIONS_V4;
use crate::version::dpp_versions::dpp_voting_versions::v2::VOTING_VERSION_V2;
use crate::version::dpp_versions::DPPVersion;
use crate::version::drive_abci_versions::drive_abci_checkpoint_parameters::v1::DRIVE_ABCI_CHECKPOINT_PARAMETERS_V1;
use crate::version::drive_abci_versions::drive_abci_method_versions::v9::DRIVE_ABCI_METHOD_VERSIONS_V9;
use crate::version::drive_abci_versions::drive_abci_query_versions::v1::DRIVE_ABCI_QUERY_VERSIONS_V1;
use crate::version::drive_abci_versions::drive_abci_structure_versions::v1::DRIVE_ABCI_STRUCTURE_VERSIONS_V1;
use crate::version::drive_abci_versions::drive_abci_validation_versions::v9::DRIVE_ABCI_VALIDATION_VERSIONS_V9;
use crate::version::drive_abci_versions::drive_abci_withdrawal_constants::v2::DRIVE_ABCI_WITHDRAWAL_CONSTANTS_V2;
use crate::version::drive_abci_versions::DriveAbciVersion;
use crate::version::drive_versions::v9::DRIVE_VERSION_V9;
use crate::version::fee::v2::FEE_VERSION2;
use crate::version::protocol_version::PlatformVersion;
use crate::version::system_data_contract_versions::v3::SYSTEM_DATA_CONTRACT_VERSIONS_V3;
use crate::version::system_limits::v3::SYSTEM_LIMITS_V3;
use crate::version::ProtocolVersion;

pub const PROTOCOL_VERSION_14: ProtocolVersion = 14;

/// v14 fixes the shared-prefix aggregate index defect: a data contract
/// declaring an aggregating (countable / summable) index that terminates at
/// a property which is also the prefix of a compound index (e.g. summable
/// `[a]` next to `[a, b]`) registered successfully but rejected every
/// document insert for most flag combinations, because Drive could not
/// legally hang the compound continuation tree under the aggregating
/// per-value tree.
///
/// `DRIVE_VERSION_V9` (via `DRIVE_DOCUMENT_METHOD_VERSIONS_V4`) bumps the
/// four document index walkers (insert/delete x top-level/recursive) to v2:
/// tree types derive through a shared continuation-demotion helper (provable
/// count-bearing value trees with compound continuations demote to
/// `CountSumTree`, since grovedb rejects count-suppressed children under
/// provable count parents by design) and continuation inserts route through
/// the completed zero-contribution wrapper matrix (`NonCounted` for non-sum
/// continuations under count-sum parents, unwrapped inserts under sum-only
/// parents, and so on). No state migration is needed: shapes without
/// compound continuations produce bit-identical operations, the broken
/// shapes could never hold documents, and the one previously-insertable
/// shape the demotion changes (a provable count-bearing value tree whose
/// continuations were all sum-bearing — insertable pre-v14 only through an
/// unenforced grovedb batch guard) simply gets `CountSumTree` value trees
/// for values first seen at v14+, which readers treat identically.
///
/// Everything else matches v13.
pub const PLATFORM_V14: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_14,
    drive: DRIVE_VERSION_V9, // changed: v2 index walkers — shared-prefix aggregate indexes become insertable
    drive_abci: DriveAbciVersion {
        structs: DRIVE_ABCI_STRUCTURE_VERSIONS_V1,
        methods: DRIVE_ABCI_METHOD_VERSIONS_V9,
        validation_and_processing: DRIVE_ABCI_VALIDATION_VERSIONS_V9,
        withdrawal_constants: DRIVE_ABCI_WITHDRAWAL_CONSTANTS_V2,
        query: DRIVE_ABCI_QUERY_VERSIONS_V1,
        checkpoints: DRIVE_ABCI_CHECKPOINT_PARAMETERS_V1,
    },
    dpp: DPPVersion {
        costs: DPP_COSTS_VERSIONS_V1,
        validation: DPP_VALIDATION_VERSIONS_V4,
        state_transition_serialization_versions: STATE_TRANSITION_SERIALIZATION_VERSIONS_V2,
        state_transition_conversion_versions: STATE_TRANSITION_CONVERSION_VERSIONS_V2,
        state_transition_method_versions: STATE_TRANSITION_METHOD_VERSIONS_V2, // changed: accepts DIP-33 payment key purposes
        state_transitions: STATE_TRANSITION_VERSIONS_V3,
        contract_versions: CONTRACT_VERSIONS_V5,
        document_versions: DOCUMENT_VERSIONS_V3,
        identity_versions: IDENTITY_VERSIONS_V1,
        voting_versions: VOTING_VERSION_V2,
        token_versions: TOKEN_VERSIONS_V2,
        asset_lock_versions: DPP_ASSET_LOCK_VERSIONS_V1,
        methods: DPP_METHOD_VERSIONS_V2,
        factory_versions: DPP_FACTORY_VERSIONS_V1,
    },
    system_data_contracts: SYSTEM_DATA_CONTRACT_VERSIONS_V3, // changed: DashPay v2 adds profile payment address fields (DIP-33)
    fee_version: FEE_VERSION2,
    system_limits: SYSTEM_LIMITS_V3,
    consensus: ConsensusVersions {
        tenderdash_consensus_version: 1,
    },
};
