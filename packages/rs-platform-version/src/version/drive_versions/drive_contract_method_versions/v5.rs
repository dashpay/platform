use crate::version::drive_versions::drive_contract_method_versions::v4::DRIVE_CONTRACT_METHOD_VERSIONS_V4;
use crate::version::drive_versions::drive_contract_method_versions::{
    DriveContractMethodVersions, DriveContractModerationMethodVersions,
};

/// Drive contract methods for protocol version 15.
///
/// Relative to [`super::v4::DRIVE_CONTRACT_METHOD_VERSIONS_V4`], the four moderation
/// writers that can free moderator-flagged bytes (`remove_contract_ban`,
/// `remove_contract_suspension`, `remove_contract_warnings`, and `add_contract_suspension`,
/// whose replacement of an existing entry may shrink it) are bumped to `1`: when the caller passes no transaction,
/// the fee-returning wrapper writes and prices inside one owned transaction and commits it
/// only once `Drive::calculate_fee` succeeded. Pricing an owner-attributed storage removal
/// without the fee history is an error from this version, and generation 0 committed the
/// write before that error surfaced. The operation builders are unchanged.
pub const DRIVE_CONTRACT_METHOD_VERSIONS_V5: DriveContractMethodVersions =
    DriveContractMethodVersions {
        moderation: DriveContractModerationMethodVersions {
            remove_contract_ban: 1,
            add_contract_suspension: 1,
            remove_contract_suspension: 1,
            remove_contract_warnings: 1,
            ..DRIVE_CONTRACT_METHOD_VERSIONS_V4.moderation
        },
        ..DRIVE_CONTRACT_METHOD_VERSIONS_V4
    };
