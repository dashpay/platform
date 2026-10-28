use crate::version::drive_versions::drive_contract_method_versions::v5::DRIVE_CONTRACT_METHOD_VERSIONS_V5;
use crate::version::drive_versions::v9::DRIVE_VERSION_V9;
use crate::version::drive_versions::{DriveMethodVersions, DriveVersion};

/// Drive version 10, introduced in protocol v15. Differs from v9 only in
/// `DRIVE_CONTRACT_METHOD_VERSIONS_V5`: applying a contract writes nothing when its format
/// version 1 bytes match the stored contract's, whatever format the stored one is in, as
/// contracts are stored compiled from this version (`contract_storage_serialization_version`
/// 2).
pub const DRIVE_VERSION_V10: DriveVersion = DriveVersion {
    methods: DriveMethodVersions {
        contract: DRIVE_CONTRACT_METHOD_VERSIONS_V5, // changed
        ..DRIVE_VERSION_V9.methods
    },
    ..DRIVE_VERSION_V9
};
