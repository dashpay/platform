use crate::version::consensus_versions::ConsensusVersions;
use crate::version::dpp_versions::dpp_asset_lock_versions::v1::DPP_ASSET_LOCK_VERSIONS_V1;
use crate::version::dpp_versions::dpp_contract_versions::v6::CONTRACT_VERSIONS_V6;
use crate::version::dpp_versions::dpp_costs_versions::v1::DPP_COSTS_VERSIONS_V1;
use crate::version::dpp_versions::dpp_document_versions::v4::DOCUMENT_VERSIONS_V4;
use crate::version::dpp_versions::dpp_factory_versions::v1::DPP_FACTORY_VERSIONS_V1;
use crate::version::dpp_versions::dpp_identity_versions::v1::IDENTITY_VERSIONS_V1;
use crate::version::dpp_versions::dpp_method_versions::v3::DPP_METHOD_VERSIONS_V3;
use crate::version::dpp_versions::dpp_state_transition_conversion_versions::v2::STATE_TRANSITION_CONVERSION_VERSIONS_V2;
use crate::version::dpp_versions::dpp_state_transition_method_versions::v1::STATE_TRANSITION_METHOD_VERSIONS_V1;
use crate::version::dpp_versions::dpp_state_transition_serialization_versions::v3::STATE_TRANSITION_SERIALIZATION_VERSIONS_V3;
use crate::version::dpp_versions::dpp_state_transition_versions::v3::STATE_TRANSITION_VERSIONS_V3;
use crate::version::dpp_versions::dpp_token_versions::v2::TOKEN_VERSIONS_V2;
use crate::version::dpp_versions::dpp_validation_versions::v5::DPP_VALIDATION_VERSIONS_V5;
use crate::version::dpp_versions::dpp_voting_versions::v2::VOTING_VERSION_V2;
use crate::version::dpp_versions::DPPVersion;
use crate::version::drive_abci_versions::drive_abci_checkpoint_parameters::v1::DRIVE_ABCI_CHECKPOINT_PARAMETERS_V1;
use crate::version::drive_abci_versions::drive_abci_method_versions::v11::DRIVE_ABCI_METHOD_VERSIONS_V11;
use crate::version::drive_abci_versions::drive_abci_query_versions::v3::DRIVE_ABCI_QUERY_VERSIONS_V3;
use crate::version::drive_abci_versions::drive_abci_state_sync_versions::v1::DRIVE_ABCI_STATE_SYNC_VERSIONS_V1;
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

pub const PROTOCOL_VERSION_15: ProtocolVersion = 15;

/// v15 enables ABCI state sync: a fresh node can bootstrap from a peer's grovedb
/// snapshot instead of replaying the chain.
///
/// The consensus changes gate on `DRIVE_ABCI_METHOD_VERSIONS_V11`:
///
/// * `run_block_proposal` 0 -> 1: every block writes a reduced platform state
///   (`Misc/reduced_saved_state`) into the replicated state just before the root hash is
///   computed, and `validator_set_update` moves above the root-hash computation so the
///   stored reduced state reflects the post-rotation validator set. The full platform
///   state only lives in non-replicated aux storage, so without this a state-synced node
///   would have no way to rebuild its in-memory state.
/// * `consensus_params_update` 1 -> 2: the first block of v15 also emits evidence
///   params sized for state-synced nodes that do not hold full history (issue #2512).
/// * `perform_events_on_first_block_of_protocol_change` writes the initial reduced state
///   at the v15 activation block, so every snapshot taken at or after activation is
///   restorable. Snapshots from before activation lack the key and are not served.
///
/// Everything else matches v14. The grovedb state sync protocol version used for
/// snapshots is `DRIVE_ABCI_STATE_SYNC_VERSIONS_V1.protocol_version` (1), shared by all
/// platform versions; grovedb updates its replication protocol in place, so exactly one
/// version exists.
pub const PLATFORM_V15: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_15,
    drive: DRIVE_VERSION_V9,
    drive_abci: DriveAbciVersion {
        structs: DRIVE_ABCI_STRUCTURE_VERSIONS_V1,
        methods: DRIVE_ABCI_METHOD_VERSIONS_V11, // changed: run_block_proposal v1 (reduced state write + validator rotation above root hash) and consensus_params_update v2 (evidence params on the v15 activation block)
        validation_and_processing: DRIVE_ABCI_VALIDATION_VERSIONS_V10,
        withdrawal_constants: DRIVE_ABCI_WITHDRAWAL_CONSTANTS_V3,
        query: DRIVE_ABCI_QUERY_VERSIONS_V3,
        checkpoints: DRIVE_ABCI_CHECKPOINT_PARAMETERS_V1,
        state_sync: DRIVE_ABCI_STATE_SYNC_VERSIONS_V1,
    },
    dpp: DPPVersion {
        costs: DPP_COSTS_VERSIONS_V1,
        validation: DPP_VALIDATION_VERSIONS_V5,
        state_transition_serialization_versions: STATE_TRANSITION_SERIALIZATION_VERSIONS_V3,
        state_transition_conversion_versions: STATE_TRANSITION_CONVERSION_VERSIONS_V2,
        state_transition_method_versions: STATE_TRANSITION_METHOD_VERSIONS_V1,
        state_transitions: STATE_TRANSITION_VERSIONS_V3,
        contract_versions: CONTRACT_VERSIONS_V6,
        document_versions: DOCUMENT_VERSIONS_V4,
        identity_versions: IDENTITY_VERSIONS_V1,
        voting_versions: VOTING_VERSION_V2,
        token_versions: TOKEN_VERSIONS_V2,
        asset_lock_versions: DPP_ASSET_LOCK_VERSIONS_V1,
        methods: DPP_METHOD_VERSIONS_V3,
        factory_versions: DPP_FACTORY_VERSIONS_V1,
    },
    system_data_contracts: SYSTEM_DATA_CONTRACT_VERSIONS_V3,
    fee_version: FEE_VERSION2,
    system_limits: SYSTEM_LIMITS_V4,
    consensus: ConsensusVersions {
        tenderdash_consensus_version: 1,
    },
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::version::v14::PLATFORM_V14;

    /// The state sync consensus changes live in v15's own method table, so a v14 node
    /// keeps running run_block_proposal v0 (no reduced-state write, rotation after the
    /// root hash) and consensus_params_update v1. Making v14 non-zero here would be
    /// consensus-breaking for already-deployed nodes.
    #[test]
    fn state_sync_consensus_changes_gate_at_v15() {
        assert_eq!(PLATFORM_V14.drive_abci.methods.engine.run_block_proposal, 0);
        assert_eq!(
            PLATFORM_V14
                .drive_abci
                .methods
                .engine
                .consensus_params_update,
            1
        );
        assert_eq!(PLATFORM_V15.drive_abci.methods.engine.run_block_proposal, 1);
        assert_eq!(
            PLATFORM_V15
                .drive_abci
                .methods
                .engine
                .consensus_params_update,
            2
        );
    }

    /// All platform versions share grovedb state sync protocol version 1 — the only
    /// version that exists, since grovedb updates its replication protocol in place.
    /// The supported set lives next to the snapshot types in drive-abci.
    #[test]
    fn state_sync_wire_protocol_version_is_one() {
        assert_eq!(PLATFORM_V15.drive_abci.state_sync.protocol_version, 1);
        assert_eq!(PLATFORM_V14.drive_abci.state_sync.protocol_version, 1);
    }
}
