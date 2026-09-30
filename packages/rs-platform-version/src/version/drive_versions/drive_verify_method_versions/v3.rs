use crate::version::drive_versions::drive_verify_method_versions::v2::DRIVE_VERIFY_METHOD_VERSIONS_V2;
use crate::version::drive_versions::drive_verify_method_versions::{
    DriveVerifyMethodVersions, DriveVerifyStateTransitionMethodVersions,
};

/// Version 3 of the Drive verify method versions.
///
/// Changed from v2: `verify_state_transition_was_executed_with_proof` 0 → 1. Feature
/// version 1 verifies a document batch proof that carries the owner's credit balance next
/// to the document (one strict verification of the merged query); feature version 0
/// verifies the document alone. Must move in lockstep with
/// `DriveProveMethodVersions::prove_state_transition`, which selects the matching prover
/// on the server side.
pub const DRIVE_VERIFY_METHOD_VERSIONS_V3: DriveVerifyMethodVersions = DriveVerifyMethodVersions {
    state_transition: DriveVerifyStateTransitionMethodVersions {
        verify_state_transition_was_executed_with_proof: 1,
    },
    ..DRIVE_VERIFY_METHOD_VERSIONS_V2
};
