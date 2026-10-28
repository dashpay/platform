use crate::version::drive_versions::drive_verify_method_versions::v3::DRIVE_VERIFY_METHOD_VERSIONS_V3;
use crate::version::drive_versions::drive_verify_method_versions::{
    DriveVerifyMethodVersions, DriveVerifyVoteMethodVersions,
};

/// Version 4 of the Drive verify method versions, introduced in protocol version 15 with the
/// yes/no poll kind: `voting.verify_yes_no_vote_poll_state_proof` and
/// `voting.verify_masternode_yes_no_vote` are `Some(0)` (`None`, not active, in every earlier
/// table). Everything else matches `DRIVE_VERIFY_METHOD_VERSIONS_V3`.
pub const DRIVE_VERIFY_METHOD_VERSIONS_V4: DriveVerifyMethodVersions = DriveVerifyMethodVersions {
    voting: DriveVerifyVoteMethodVersions {
        verify_yes_no_vote_poll_state_proof: Some(0),
        verify_masternode_yes_no_vote: Some(0),
        ..DRIVE_VERIFY_METHOD_VERSIONS_V3.voting
    },
    ..DRIVE_VERIFY_METHOD_VERSIONS_V3
};
