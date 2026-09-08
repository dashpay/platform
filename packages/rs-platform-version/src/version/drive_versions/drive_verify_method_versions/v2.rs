use crate::version::drive_versions::drive_verify_method_versions::v1::DRIVE_VERIFY_METHOD_VERSIONS_V1;
use crate::version::drive_versions::drive_verify_method_versions::{
    DriveVerifyAddressFundsMethodVersions, DriveVerifyMethodVersions,
};

/// Version 2 of the Drive verify method versions.
///
/// Changed from v1: `verify_compacted_address_balance_changes` 0 → 1. Feature
/// version 0 decodes the legacy single GroveDB proof; feature version 1
/// decodes the two-proof `CompactedAddressBalanceProof` bincode envelope
/// whose independently verified predecessor proof binds the forward-query
/// start key (the soundness fix from the bind-and-bound proof decoding
/// change). Must move in lockstep with
/// `DriveSavedBlockTransactionsMethodVersions::prove_compacted_address_balance_changes`,
/// which selects the matching encoder on the server side.
pub const DRIVE_VERIFY_METHOD_VERSIONS_V2: DriveVerifyMethodVersions = DriveVerifyMethodVersions {
    address_funds: DriveVerifyAddressFundsMethodVersions {
        verify_address_info: 0,
        verify_addresses_infos: 0,
        verify_address_funds_trunk_query: 0,
        verify_address_funds_branch_query: 0,
        verify_recent_address_balance_changes: 0,
        verify_compacted_address_balance_changes: 1,
    },
    ..DRIVE_VERIFY_METHOD_VERSIONS_V1
};
