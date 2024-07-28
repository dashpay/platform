use crate::state_transition::state_transitions::identity::masternode_vote_transition::v0::MasternodeVoteTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for MasternodeVoteTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
