use crate::state_transition::state_transitions::identity::masternode_vote_transition::MasternodeVoteTransition;
use crate::state_transition::FeatureVersioned;
use versioned_feature_core::FeatureVersion;

impl FeatureVersioned for MasternodeVoteTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            MasternodeVoteTransition::V0(v0) => v0.feature_version(),
        }
    }
}
