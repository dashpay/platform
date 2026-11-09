use crate::version::drive_versions::drive_grove_method_versions::v1::DRIVE_GROVE_METHOD_VERSIONS_V1;
use crate::version::drive_versions::drive_grove_method_versions::{
    DriveGroveApplyMethodVersions, DriveGroveMethodVersions,
};

/// V2 is protocol version 15's GroveDB method table. Relative to V1,
/// `apply.grove_apply_batch` is 1: in a batch updating documents, an element keeps its stored
/// bytes under the owner the written flags name, and a gas sponsor's owner changes are
/// accounted as replacements (see protocol version 15's notes). Estimates do not change: a document
/// update is already estimated as the insertion of everything it writes.
pub const DRIVE_GROVE_METHOD_VERSIONS_V2: DriveGroveMethodVersions = DriveGroveMethodVersions {
    apply: DriveGroveApplyMethodVersions {
        grove_apply_batch: 1, // changed: the flags update follows the written owner
        ..DRIVE_GROVE_METHOD_VERSIONS_V1.apply
    },
    ..DRIVE_GROVE_METHOD_VERSIONS_V1
};
