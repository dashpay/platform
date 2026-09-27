use crate::version::consensus_versions::ConsensusVersions;
use crate::version::dpp_versions::dpp_asset_lock_versions::v1::DPP_ASSET_LOCK_VERSIONS_V1;
use crate::version::dpp_versions::dpp_contract_versions::v6::CONTRACT_VERSIONS_V6;
use crate::version::dpp_versions::dpp_costs_versions::v1::DPP_COSTS_VERSIONS_V1;
use crate::version::dpp_versions::dpp_document_versions::v4::DOCUMENT_VERSIONS_V4;
use crate::version::dpp_versions::dpp_factory_versions::v1::DPP_FACTORY_VERSIONS_V1;
use crate::version::dpp_versions::dpp_identity_versions::v1::IDENTITY_VERSIONS_V1;
use crate::version::dpp_versions::dpp_method_versions::v3::DPP_METHOD_VERSIONS_V3;
use crate::version::dpp_versions::dpp_state_transition_conversion_versions::v2::STATE_TRANSITION_CONVERSION_VERSIONS_V2;
use crate::version::dpp_versions::dpp_state_transition_method_versions::v2::STATE_TRANSITION_METHOD_VERSIONS_V2;
use crate::version::dpp_versions::dpp_state_transition_serialization_versions::v3::STATE_TRANSITION_SERIALIZATION_VERSIONS_V3;
use crate::version::dpp_versions::dpp_state_transition_versions::v4::STATE_TRANSITION_VERSIONS_V4;
use crate::version::dpp_versions::dpp_token_versions::v3::TOKEN_VERSIONS_V3;
use crate::version::dpp_versions::dpp_validation_versions::v5::DPP_VALIDATION_VERSIONS_V5;
use crate::version::dpp_versions::dpp_voting_versions::v2::VOTING_VERSION_V2;
use crate::version::dpp_versions::DPPVersion;
use crate::version::drive_abci_versions::drive_abci_checkpoint_parameters::v1::DRIVE_ABCI_CHECKPOINT_PARAMETERS_V1;
use crate::version::drive_abci_versions::drive_abci_method_versions::v10::DRIVE_ABCI_METHOD_VERSIONS_V10;
use crate::version::drive_abci_versions::drive_abci_query_versions::v3::DRIVE_ABCI_QUERY_VERSIONS_V3;
use crate::version::drive_abci_versions::drive_abci_structure_versions::v2::DRIVE_ABCI_STRUCTURE_VERSIONS_V2;
use crate::version::drive_abci_versions::drive_abci_validation_versions::v10::DRIVE_ABCI_VALIDATION_VERSIONS_V10;
use crate::version::drive_abci_versions::drive_abci_withdrawal_constants::v3::DRIVE_ABCI_WITHDRAWAL_CONSTANTS_V3;
use crate::version::drive_abci_versions::DriveAbciVersion;
use crate::version::drive_versions::v9::DRIVE_VERSION_V9;
use crate::version::fee::v3::FEE_VERSION3;
use crate::version::protocol_version::PlatformVersion;
use crate::version::system_data_contract_versions::v3::SYSTEM_DATA_CONTRACT_VERSIONS_V3;
use crate::version::system_limits::v4::SYSTEM_LIMITS_V4;
use crate::version::ProtocolVersion;

pub const PROTOCOL_VERSION_15: ProtocolVersion = 15;

/// Introduced as the activation gate for the consensus changes of the 4.3
/// line. Functionally identical to v14 at introduction: the same component
/// version structs, no behavior change. Each change that needs this gate
/// lands in its own follow-up and bumps the component table it consumes here;
/// keeping v15 == v14 until then lets mixed-version validators agree.
pub const PLATFORM_V15: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_15,
    drive: DRIVE_VERSION_V9,
    drive_abci: DriveAbciVersion {
        structs: DRIVE_ABCI_STRUCTURE_VERSIONS_V2,
        methods: DRIVE_ABCI_METHOD_VERSIONS_V10,
        validation_and_processing: DRIVE_ABCI_VALIDATION_VERSIONS_V10,
        withdrawal_constants: DRIVE_ABCI_WITHDRAWAL_CONSTANTS_V3,
        query: DRIVE_ABCI_QUERY_VERSIONS_V3,
        checkpoints: DRIVE_ABCI_CHECKPOINT_PARAMETERS_V1,
    },
    dpp: DPPVersion {
        costs: DPP_COSTS_VERSIONS_V1,
        validation: DPP_VALIDATION_VERSIONS_V5,
        state_transition_serialization_versions: STATE_TRANSITION_SERIALIZATION_VERSIONS_V3,
        state_transition_conversion_versions: STATE_TRANSITION_CONVERSION_VERSIONS_V2,
        state_transition_method_versions: STATE_TRANSITION_METHOD_VERSIONS_V2,
        state_transitions: STATE_TRANSITION_VERSIONS_V4,
        contract_versions: CONTRACT_VERSIONS_V6,
        document_versions: DOCUMENT_VERSIONS_V4,
        identity_versions: IDENTITY_VERSIONS_V1,
        voting_versions: VOTING_VERSION_V2,
        token_versions: TOKEN_VERSIONS_V3,
        asset_lock_versions: DPP_ASSET_LOCK_VERSIONS_V1,
        methods: DPP_METHOD_VERSIONS_V3,
        factory_versions: DPP_FACTORY_VERSIONS_V1,
    },
    system_data_contracts: SYSTEM_DATA_CONTRACT_VERSIONS_V3,
    fee_version: FEE_VERSION3,
    system_limits: SYSTEM_LIMITS_V4,
    consensus: ConsensusVersions {
        tenderdash_consensus_version: 1,
    },
};
