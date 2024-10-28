use crate::version::consensus_versions::ConsensusVersions;
use crate::version::dpp_versions::dpp_asset_lock_versions::v1::DPP_ASSET_LOCK_VERSIONS_V1;
use crate::version::dpp_versions::dpp_contract_versions::v1::CONTRACT_VERSIONS_V1;
use crate::version::dpp_versions::dpp_costs_versions::v1::DPP_COSTS_VERSIONS_V1;
use crate::version::dpp_versions::dpp_document_versions::v1::DOCUMENT_VERSIONS_V1;
use crate::version::dpp_versions::dpp_factory_versions::v1::DPP_FACTORY_VERSIONS_V1;
use crate::version::dpp_versions::dpp_identity_versions::v1::IDENTITY_VERSIONS_V1;
use crate::version::dpp_versions::dpp_method_versions::v1::DPP_METHOD_VERSIONS_V1;
use crate::version::dpp_versions::dpp_state_transition_conversion_versions::v2::STATE_TRANSITION_CONVERSION_VERSIONS_V2;
use crate::version::dpp_versions::dpp_state_transition_method_versions::v1::STATE_TRANSITION_METHOD_VERSIONS_V1;
use crate::version::dpp_versions::dpp_state_transition_serialization_versions::v1::STATE_TRANSITION_SERIALIZATION_VERSIONS_V1;
use crate::version::dpp_versions::dpp_state_transition_versions::v2::STATE_TRANSITION_VERSIONS_V2;
use crate::version::dpp_versions::dpp_validation_versions::v2::DPP_VALIDATION_VERSIONS_V2;
use crate::version::dpp_versions::dpp_voting_versions::v2::VOTING_VERSION_V2;
use crate::version::dpp_versions::DPPVersion;
use crate::version::drive_abci_versions::drive_abci_method_versions::v2::DRIVE_ABCI_METHOD_VERSIONS_V2;
use crate::version::drive_abci_versions::drive_abci_query_versions::v1::DRIVE_ABCI_QUERY_VERSIONS_V1;
use crate::version::drive_abci_versions::drive_abci_structure_versions::v1::DRIVE_ABCI_STRUCTURE_VERSIONS_V1;
use crate::version::drive_abci_versions::drive_abci_validation_versions::v2::DRIVE_ABCI_VALIDATION_VERSIONS_V2;
use crate::version::drive_abci_versions::drive_abci_withdrawal_constants::v1::DRIVE_ABCI_WITHDRAWAL_CONSTANTS_V1;
use crate::version::drive_abci_versions::DriveAbciVersion;
use crate::version::drive_versions::v2::DRIVE_VERSION_V2;
use crate::version::fee::v1::FEE_VERSION1;
use crate::version::protocol_version::PlatformVersion;
use crate::version::system_data_contract_versions::v1::SYSTEM_DATA_CONTRACT_VERSIONS_V1;
use crate::version::system_limits::v1::SYSTEM_LIMITS_V1;
use crate::version::ProtocolVersion;

pub const PROTOCOL_VERSION_3: ProtocolVersion = 3;

/// This version introduces tenderdash_consensus_version as 1.
/// We did this because of the issues in distribution for Evonodes.
/// We are setting the requirement to 51% voting for this upgrade for it to take effect.
/// This was done directly in ABCI.
/// If we get between 51 and 67% we will have a chain stall
/// However the chain will come back up as soon as enough have upgraded.

pub const PLATFORM_V3: PlatformVersion = PlatformVersion {
    protocol_version: 3,
    drive: DRIVE_VERSION_V2,
    drive_abci: DriveAbciVersion {
        structs: DRIVE_ABCI_STRUCTURE_VERSIONS_V1,
        methods: DRIVE_ABCI_METHOD_VERSIONS_V2,
        validation_and_processing: DRIVE_ABCI_VALIDATION_VERSIONS_V2,
        withdrawal_constants: DRIVE_ABCI_WITHDRAWAL_CONSTANTS_V1,
        query: DRIVE_ABCI_QUERY_VERSIONS_V1,
    },
    dpp: DPPVersion {
        costs: DPP_COSTS_VERSIONS_V1,
        validation: DPP_VALIDATION_VERSIONS_V2,
        state_transition_serialization_versions: STATE_TRANSITION_SERIALIZATION_VERSIONS_V1,
        state_transition_conversion_versions: STATE_TRANSITION_CONVERSION_VERSIONS_V2,
        state_transition_method_versions: STATE_TRANSITION_METHOD_VERSIONS_V1,
        state_transitions: STATE_TRANSITION_VERSIONS_V2,
        contract_versions: CONTRACT_VERSIONS_V1,
        document_versions: DOCUMENT_VERSIONS_V1,
        identity_versions: IDENTITY_VERSIONS_V1,
        voting_versions: VOTING_VERSION_V2,
        asset_lock_versions: DPP_ASSET_LOCK_VERSIONS_V1,
        methods: DPP_METHOD_VERSIONS_V1,
        factory_versions: DPP_FACTORY_VERSIONS_V1,
    },
    system_data_contracts: SYSTEM_DATA_CONTRACT_VERSIONS_V1,
    fee_version: FEE_VERSION1,
    system_limits: SYSTEM_LIMITS_V1,
    consensus: ConsensusVersions {
        tenderdash_consensus_version: 1,
    },
};
