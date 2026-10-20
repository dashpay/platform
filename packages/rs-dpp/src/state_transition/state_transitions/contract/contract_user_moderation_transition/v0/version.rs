use crate::state_transition::contract_user_moderation_transition::v0::ContractUserModerationTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for ContractUserModerationTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
