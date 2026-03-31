use crate::version::consensus_versions::ConsensusVersions;
use crate::version::dpp_versions::dpp_asset_lock_versions::v1::DPP_ASSET_LOCK_VERSIONS_V1;
use crate::version::dpp_versions::dpp_contract_versions::v3::CONTRACT_VERSIONS_V3;
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
use crate::version::dpp_versions::dpp_validation_versions::v3::DPP_VALIDATION_VERSIONS_V3;
use crate::version::dpp_versions::dpp_voting_versions::v2::VOTING_VERSION_V2;
use crate::version::dpp_versions::DPPVersion;
use crate::version::drive_abci_versions::drive_abci_checkpoint_parameters::v1::DRIVE_ABCI_CHECKPOINT_PARAMETERS_V1;
use crate::version::drive_abci_versions::drive_abci_method_versions::v7::DRIVE_ABCI_METHOD_VERSIONS_V7;
use crate::version::drive_abci_versions::drive_abci_query_versions::v1::DRIVE_ABCI_QUERY_VERSIONS_V1;
use crate::version::drive_abci_versions::drive_abci_structure_versions::v1::DRIVE_ABCI_STRUCTURE_VERSIONS_V1;
use crate::version::drive_abci_versions::drive_abci_validation_versions::v8::DRIVE_ABCI_VALIDATION_VERSIONS_V8;
use crate::version::drive_abci_versions::drive_abci_withdrawal_constants::v2::DRIVE_ABCI_WITHDRAWAL_CONSTANTS_V2;
use crate::version::drive_abci_versions::DriveAbciVersion;
use crate::version::drive_versions::v7::DRIVE_VERSION_V7;
use crate::version::fee::v2::FEE_VERSION2;
use crate::version::protocol_version::PlatformVersion;
use crate::version::system_data_contract_versions::v1::SYSTEM_DATA_CONTRACT_VERSIONS_V1;
use crate::version::system_limits::v1::SYSTEM_LIMITS_V1;
use crate::version::ProtocolVersion;

pub const PROTOCOL_VERSION_12: ProtocolVersion = 12;

/// This version is for Platform release 3.1.0
pub const PLATFORM_V12: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_12,
    drive: DRIVE_VERSION_V7, // changed: shielded pool (commitment tree, nullifiers, anchors, address funds, sinsemilla hashing)
    drive_abci: DriveAbciVersion {
        structs: DRIVE_ABCI_STRUCTURE_VERSIONS_V1,
        methods: DRIVE_ABCI_METHOD_VERSIONS_V7,
        // enforce minimum config version check for contract create/update transitions
        validation_and_processing: DRIVE_ABCI_VALIDATION_VERSIONS_V8,
        withdrawal_constants: DRIVE_ABCI_WITHDRAWAL_CONSTANTS_V2,
        query: DRIVE_ABCI_QUERY_VERSIONS_V1,
        checkpoints: DRIVE_ABCI_CHECKPOINT_PARAMETERS_V1,
    },
    dpp: DPPVersion {
        costs: DPP_COSTS_VERSIONS_V1,
        // prevent sized_integer_types config downgrade on contract update
        validation: DPP_VALIDATION_VERSIONS_V3,
        state_transition_serialization_versions: STATE_TRANSITION_SERIALIZATION_VERSIONS_V2,
        state_transition_conversion_versions: STATE_TRANSITION_CONVERSION_VERSIONS_V2,
        state_transition_method_versions: STATE_TRANSITION_METHOD_VERSIONS_V1,
        state_transitions: STATE_TRANSITION_VERSIONS_V3,
        contract_versions: CONTRACT_VERSIONS_V3,
        document_versions: DOCUMENT_VERSIONS_V3,
        identity_versions: IDENTITY_VERSIONS_V1,
        voting_versions: VOTING_VERSION_V2,
        token_versions: TOKEN_VERSIONS_V2, // fixes action_id vote-swap for config update + set price
        asset_lock_versions: DPP_ASSET_LOCK_VERSIONS_V1,
        methods: DPP_METHOD_VERSIONS_V2,
        factory_versions: DPP_FACTORY_VERSIONS_V1,
    },
    system_data_contracts: SYSTEM_DATA_CONTRACT_VERSIONS_V1,
    fee_version: FEE_VERSION2,
    system_limits: SYSTEM_LIMITS_V1,
    consensus: ConsensusVersions {
        tenderdash_consensus_version: 1,
    },
};
