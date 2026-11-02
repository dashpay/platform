use crate::version::drive_versions::drive_contract_method_versions::v4::DRIVE_CONTRACT_METHOD_VERSIONS_V4;
use crate::version::drive_versions::drive_contract_method_versions::{
    DriveContractApplyMethodVersions, DriveContractMethodVersions,
};

/// Drive contract methods for protocol v15+.
///
/// Relative to [`super::v4::DRIVE_CONTRACT_METHOD_VERSIONS_V4`]:
///
/// * `apply.apply_contract_with_serialization` is bumped to `1`. From protocol version 15 a
///   contract is stored compiled (format version 2), so the bytes of an unchanged contract can
///   differ from the ones it was stored with in format version 1. v1 writes nothing when the
///   two contracts' format version 1 bytes match, where v0 compared the stored bytes alone and
///   would have rewritten an unchanged contract as an update.
pub const DRIVE_CONTRACT_METHOD_VERSIONS_V5: DriveContractMethodVersions =
    DriveContractMethodVersions {
        apply: DriveContractApplyMethodVersions {
            apply_contract_with_serialization: 1,
            ..DRIVE_CONTRACT_METHOD_VERSIONS_V4.apply
        },
        ..DRIVE_CONTRACT_METHOD_VERSIONS_V4
    };
