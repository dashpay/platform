use crate::version::drive_versions::drive_token_method_versions::v1::DRIVE_TOKEN_METHOD_VERSIONS_V1;
use crate::version::drive_versions::drive_token_method_versions::{
    DriveTokenDistributionMethodVersions, DriveTokenMethodVersions,
};

/// Drive token methods for protocol v14+.
///
/// Identical to [`super::v1::DRIVE_TOKEN_METHOD_VERSIONS_V1`] except
/// `distribution.add_pre_programmed_distributions` and
/// `distribution.evonode_participation_rewards` are bumped to `1`.
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
///
/// An `EvonodesByParticipation` claim weighs each cycle by the claimant's share of the blocks
/// in the finalized epochs it reads. v0 read at most the query bound of epochs but evaluated
/// the whole claimed range, so a claim reaching past them failed as an internal error (and
/// never advanced, for good) or, for a fixed amount, applied the share of the epochs read to
/// the whole range. v1 pays only through the last whole cycle it read, reads at least one
/// whole cycle (`SystemLimits::max_evonode_reward_claim_epochs` or the cycle, whichever is
/// longer), and counts an epoch without finalized info below the last one read as an epoch
/// in which no block was produced.
pub const DRIVE_TOKEN_METHOD_VERSIONS_V2: DriveTokenMethodVersions = DriveTokenMethodVersions {
    distribution: DriveTokenDistributionMethodVersions {
        add_pre_programmed_distributions: 1,
        evonode_participation_rewards: 1,
        ..DRIVE_TOKEN_METHOD_VERSIONS_V1.distribution
    },
    ..DRIVE_TOKEN_METHOD_VERSIONS_V1
};
