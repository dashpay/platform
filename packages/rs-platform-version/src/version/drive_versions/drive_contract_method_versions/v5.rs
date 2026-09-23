use crate::version::drive_versions::drive_contract_method_versions::v4::DRIVE_CONTRACT_METHOD_VERSIONS_V4;
use crate::version::drive_versions::drive_contract_method_versions::{
    DriveContractApplyMethodVersions, DriveContractMethodVersions,
    DriveContractModerationMethodVersions,
};

/// Drive contract methods for protocol version 15.
///
/// Relative to [`super::v4::DRIVE_CONTRACT_METHOD_VERSIONS_V4`]:
///
/// * `apply.apply_contract_with_serialization` is bumped to `1`. From protocol version 15 a
///   contract is stored compiled (format version 2), so the bytes of an unchanged contract can
///   differ from the ones it was stored with in format version 1. v1 writes nothing when the
///   two contracts' format version 1 bytes match, where v0 compared the stored bytes alone and
///   would have rewritten an unchanged contract as an update.
/// * The four moderation writers that can free moderator-flagged bytes (`remove_contract_ban`,
///   `remove_contract_suspension`, `remove_contract_warnings`, and `add_contract_suspension`,
///   whose replacement of an existing entry may shrink it) are bumped to `1`: when the caller
///   passes no transaction, the fee-returning wrapper writes and prices inside one owned
///   transaction and commits it only once `Drive::calculate_fee` succeeded. Pricing an
///   owner-attributed storage removal without the fee history is an error from this version,
///   and generation 0 committed the write before that error surfaced. The operation builders
///   are unchanged.
pub const DRIVE_CONTRACT_METHOD_VERSIONS_V5: DriveContractMethodVersions =
    DriveContractMethodVersions {
        apply: DriveContractApplyMethodVersions {
            apply_contract_with_serialization: 1,
            ..DRIVE_CONTRACT_METHOD_VERSIONS_V4.apply
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
