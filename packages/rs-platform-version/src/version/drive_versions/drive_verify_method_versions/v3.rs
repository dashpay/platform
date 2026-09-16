use crate::version::drive_versions::drive_verify_method_versions::v2::DRIVE_VERIFY_METHOD_VERSIONS_V2;
use crate::version::drive_versions::drive_verify_method_versions::{
    DriveVerifyMethodVersions, DriveVerifyStateTransitionMethodVersions,
};

/// Version 3 of the Drive verify method versions.
///
/// Changed from v2: `verify_state_transition_was_executed_with_proof` 0 -> 1.
/// Generation 1 verifies a delta-based (V1) data contract update by applying
/// the delta to the pre-update contract the known-contracts provider supplies
/// and comparing the whole result with the proven contract. Every other
/// transition kind verifies exactly as in generation 0.
pub const DRIVE_VERIFY_METHOD_VERSIONS_V3: DriveVerifyMethodVersions = DriveVerifyMethodVersions {
    state_transition: DriveVerifyStateTransitionMethodVersions {
        verify_state_transition_was_executed_with_proof: 1,
    },
    ..DRIVE_VERIFY_METHOD_VERSIONS_V2
};
