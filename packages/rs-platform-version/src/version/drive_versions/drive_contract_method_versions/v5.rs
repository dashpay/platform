use crate::version::drive_versions::drive_contract_method_versions::v4::DRIVE_CONTRACT_METHOD_VERSIONS_V4;
use crate::version::drive_versions::drive_contract_method_versions::{
    DriveContractApplyMethodVersions, DriveContractMethodVersions,
    DriveContractModerationMethodVersions, DriveContractUpdateMethodVersions,
};

/// Drive contract methods for protocol version 15.
///
/// Relative to [`super::v4::DRIVE_CONTRACT_METHOD_VERSIONS_V4`], every fee-returning
/// contract writer that can free owner- or moderator-flagged bytes gets a generation that,
/// when the caller passes no transaction, writes and prices inside one owned transaction and
/// commits it only once `Drive::calculate_fee` succeeded: `update.update_contract` 2 -> 3
/// (the element writer stays at generation 2), `apply.apply_contract_with_serialization`
/// 0 -> 1 (its operation builder stays at 0), and the four moderation writers
/// `remove_contract_ban`, `remove_contract_suspension`, `remove_contract_warnings` and
/// `add_contract_suspension` (whose replacement of an existing entry may shrink it) 0 -> 1.
/// Pricing an owner-attributed storage removal without the fee history is an error from this
/// version, and the earlier generations committed the write before that error surfaced.
pub const DRIVE_CONTRACT_METHOD_VERSIONS_V5: DriveContractMethodVersions =
    DriveContractMethodVersions {
        apply: DriveContractApplyMethodVersions {
            apply_contract_with_serialization: 1,
            ..DRIVE_CONTRACT_METHOD_VERSIONS_V4.apply
        },
        update: DriveContractUpdateMethodVersions {
            update_contract: 3,
            ..DRIVE_CONTRACT_METHOD_VERSIONS_V4.update
        },
        moderation: DriveContractModerationMethodVersions {
            remove_contract_ban: 1,
            add_contract_suspension: 1,
            remove_contract_suspension: 1,
            remove_contract_warnings: 1,
            ..DRIVE_CONTRACT_METHOD_VERSIONS_V4.moderation
        },
        ..DRIVE_CONTRACT_METHOD_VERSIONS_V4
    };
