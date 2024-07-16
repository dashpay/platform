use crate::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for MasternodeVoteTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            MasternodeVoteTransition::V0(v0) => v0.feature_version(),
        }
    }
}
