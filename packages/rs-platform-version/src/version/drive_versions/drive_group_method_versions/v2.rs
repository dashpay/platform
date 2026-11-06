use crate::version::drive_versions::drive_group_method_versions::v1::DRIVE_GROUP_METHOD_VERSIONS_V1;
use crate::version::drive_versions::drive_group_method_versions::{
    DriveGroupInsertMethodVersions, DriveGroupMethodVersions,
};

/// Drive group methods for protocol version 15.
///
/// Relative to [`super::v1::DRIVE_GROUP_METHOD_VERSIONS_V1`], `insert.add_group_action` is
/// bumped to `1`: when the caller passes no transaction, the fee-returning wrapper writes and
/// prices inside one owned transaction and commits it only once `Drive::calculate_fee`
/// succeeded. Pricing an owner-attributed storage removal without the fee history is an error
/// from this version, and generation 0 committed a closing action before that error surfaced.
/// The operation builders are unchanged.
pub const DRIVE_GROUP_METHOD_VERSIONS_V2: DriveGroupMethodVersions = DriveGroupMethodVersions {
    insert: DriveGroupInsertMethodVersions {
        add_group_action: 1, // changed in v15: the fee-returning wrapper prices before it commits its owned transaction
        ..DRIVE_GROUP_METHOD_VERSIONS_V1.insert
    },
    ..DRIVE_GROUP_METHOD_VERSIONS_V1
};
