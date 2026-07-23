use crate::version::consensus_versions::ConsensusVersions;
use crate::version::dpp_versions::dpp_asset_lock_versions::v1::DPP_ASSET_LOCK_VERSIONS_V1;
use crate::version::dpp_versions::dpp_contract_versions::v5::CONTRACT_VERSIONS_V5;
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
use crate::version::drive_abci_versions::drive_abci_query_versions::v1::DRIVE_ABCI_QUERY_VERSIONS_V1;
use crate::version::drive_abci_versions::drive_abci_structure_versions::v1::DRIVE_ABCI_STRUCTURE_VERSIONS_V1;
use crate::version::drive_abci_versions::drive_abci_validation_versions::v9::DRIVE_ABCI_VALIDATION_VERSIONS_V9;
use crate::version::drive_abci_versions::drive_abci_withdrawal_constants::v2::DRIVE_ABCI_WITHDRAWAL_CONSTANTS_V2;
use crate::version::drive_abci_versions::DriveAbciVersion;
use crate::version::drive_versions::v8::DRIVE_VERSION_V8;
use crate::version::fee::v2::FEE_VERSION2;
use crate::version::protocol_version::PlatformVersion;
use crate::version::system_data_contract_versions::v2::SYSTEM_DATA_CONTRACT_VERSIONS_V2;
use crate::version::system_limits::v2::SYSTEM_LIMITS_V2;
use crate::version::ProtocolVersion;

pub const PROTOCOL_VERSION_13: ProtocolVersion = 13;

/// v13 expands the recent per-block address-balance set that incremental client
/// sync reads: `DRIVE_ABCI_METHOD_VERSIONS_V9` bumps two fields from 0 to 1, both
/// recording what v0 dropped — (1) `record_added_balance_outputs` folds
/// shielded-spend transparent credits (Unshield net output, ShieldFromAssetLock
/// surplus, identity-create fallback), and (2) `process_validation_result`
/// records the balance effects of paid-invalid / unsuccessful-paid transitions
/// (charged fees, adjusted outputs), in a real _v1 of the helper so its _v0 stays
/// byte-identical to old nodes (the outer `process_raw_state_transitions` loop is
/// unchanged and stays at v0). The storage method
/// (`store_address_balances_to_recent_block_storage`) is unchanged — only the
/// recorded set differs. Because that set changes the committed state root, both
/// bumps activate only once the network votes in v13.
///
/// v13 also enables DPNS username transfers and sales:
/// * `DRIVE_ABCI_VALIDATION_VERSIONS_V9` bumps data trigger bindings to v1,
///   dropping the reject bindings for Transfer, Purchase and UpdatePrice on
///   DPNS `domain` documents. It also revises the shielded identity-create
///   exit-denomination set: 0.03 and 0.25 DASH are added and 0.3 DASH is
///   retired, giving 0.03 / 0.1 / 0.25 / 0.5 / 1 DASH.
/// * `DRIVE_VERSION_V8` bumps the document transfer/purchase high-level
///   operation conversions to v1, which rewrite a transferred or purchased
///   domain's `records.identity` to the new owner so the username resolves
///   to the buyer.
pub const PLATFORM_V13: PlatformVersion = PlatformVersion {
    protocol_version: PROTOCOL_VERSION_13,
    drive: DRIVE_VERSION_V8,
    drive_abci: DriveAbciVersion {
        structs: DRIVE_ABCI_STRUCTURE_VERSIONS_V1,
        methods: DRIVE_ABCI_METHOD_VERSIONS_V9, // changed: records shielded-spend transparent credits (Unshield net output, ShieldFromAssetLock surplus, identity-create fallback) into the recent per-block address-balance set
        validation_and_processing: DRIVE_ABCI_VALIDATION_VERSIONS_V9, // changed: allow DPNS domain transfer/purchase/update-price
        withdrawal_constants: DRIVE_ABCI_WITHDRAWAL_CONSTANTS_V2,
        query: DRIVE_ABCI_QUERY_VERSIONS_V1,
        checkpoints: DRIVE_ABCI_CHECKPOINT_PARAMETERS_V1,
    },
    dpp: DPPVersion {
        costs: DPP_COSTS_VERSIONS_V1,
        validation: DPP_VALIDATION_VERSIONS_V4,
        state_transition_serialization_versions: STATE_TRANSITION_SERIALIZATION_VERSIONS_V2,
        state_transition_conversion_versions: STATE_TRANSITION_CONVERSION_VERSIONS_V2,
        state_transition_method_versions: STATE_TRANSITION_METHOD_VERSIONS_V1,
        state_transitions: STATE_TRANSITION_VERSIONS_V3,
        contract_versions: CONTRACT_VERSIONS_V5, // changed: v2 document meta-schema accepts the keeps*History flags
        document_versions: DOCUMENT_VERSIONS_V3,
        identity_versions: IDENTITY_VERSIONS_V1,
        voting_versions: VOTING_VERSION_V2,
        token_versions: TOKEN_VERSIONS_V2,
        asset_lock_versions: DPP_ASSET_LOCK_VERSIONS_V1,
        methods: DPP_METHOD_VERSIONS_V2,
        factory_versions: DPP_FACTORY_VERSIONS_V1,
    },
    system_data_contracts: SYSTEM_DATA_CONTRACT_VERSIONS_V2, // changed: DPNS v2 subscribes domain to document history
    fee_version: FEE_VERSION2,
    system_limits: SYSTEM_LIMITS_V2,
    consensus: ConsensusVersions {
        tenderdash_consensus_version: 1,
    },
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::version::v12::PLATFORM_V12;

    #[test]
    fn token_authorization_hardening_activates_only_at_v13() {
        let v12_batch = &PLATFORM_V12
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition;
        let v13_batch = &PLATFORM_V13
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition;

        assert_eq!(v12_batch.token_base_transition_state_validation, 0);
        assert_eq!(v13_batch.token_base_transition_state_validation, 1);
        assert_eq!(v12_batch.token_base_transition_group_action_validation, 0);
        assert_eq!(v13_batch.token_base_transition_group_action_validation, 1);
        assert_eq!(v12_batch.token_config_update_transition_state_validation, 0);
        assert_eq!(v13_batch.token_config_update_transition_state_validation, 1);
        assert_eq!(
            PLATFORM_V12
                .dpp
                .validation
                .data_contract
                .validate_token_config_groups_exist,
            0
        );
        assert_eq!(
            PLATFORM_V13
                .dpp
                .validation
                .data_contract
                .validate_token_config_groups_exist,
            1
        );
    }
}
