use crate::version::drive_versions::drive_token_method_versions::v1::DRIVE_TOKEN_METHOD_VERSIONS_V1;
use crate::version::drive_versions::drive_token_method_versions::{
    DriveTokenDistributionMethodVersions, DriveTokenMethodVersions,
};

/// Drive token methods for protocol v14+.
///
/// Identical to [`super::v1::DRIVE_TOKEN_METHOD_VERSIONS_V1`] except
/// `distribution.add_pre_programmed_distributions` is bumped to `1`.
///
/// Every pre-programmed release is indexed under a tree keyed by its release
/// time that all tokens share. v0 looked for that tree in state only, so two
/// tokens of one contract releasing at the same time each queued its creation
/// in the same batch. A node verifying the consistency of its batches
/// (`batching_consistency_verification`, off by default) refuses that batch
/// as an internal error ("insertion order error") while every other node
/// stores the contract. v1 also looks among the operations already gathered,
/// so the tree is queued once. The stored state is the one v0 stores on a
/// default node; the processing fee drops by the existence read the later
/// tokens no longer make.
pub const DRIVE_TOKEN_METHOD_VERSIONS_V2: DriveTokenMethodVersions = DriveTokenMethodVersions {
    distribution: DriveTokenDistributionMethodVersions {
        add_pre_programmed_distributions: 1,
        ..DRIVE_TOKEN_METHOD_VERSIONS_V1.distribution
    },
    ..DRIVE_TOKEN_METHOD_VERSIONS_V1
};
