use crate::version::dpp_versions::dpp_state_transition_versions::v4::STATE_TRANSITION_VERSIONS_V4;
use crate::version::dpp_versions::dpp_state_transition_versions::{
    DPPStateTransitionVersions, IdentityTransitionAssetLockVersions, IdentityTransitionVersions,
};

/// Protocol version 15. Identical to [`super::v4::STATE_TRANSITION_VERSIONS_V4`] except that the
/// asset lock input cap moves from the transaction structure check to the instant proof
/// structure check: a chain proof's transaction, which Drive fetches from Core and the state
/// transition does not carry, is no longer held to it, while an instant proof's embedded
/// transaction still is.
pub const STATE_TRANSITION_VERSIONS_V5: DPPStateTransitionVersions = DPPStateTransitionVersions {
    identities: IdentityTransitionVersions {
        asset_locks: IdentityTransitionAssetLockVersions {
            validate_asset_lock_transaction_structure: 1, // changed: no input cap
            validate_instant_asset_lock_proof_structure: 1, // changed: caps the embedded transaction's inputs
            ..STATE_TRANSITION_VERSIONS_V4.identities.asset_locks
        },
        ..STATE_TRANSITION_VERSIONS_V4.identities
    },
    ..STATE_TRANSITION_VERSIONS_V4
};
