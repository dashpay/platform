use crate::state_transition::contract_user_moderation_transition::ContractUserModerationTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for ContractUserModerationTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            ContractUserModerationTransition::V0(v0) => v0.feature_version(),
        }
    }
}
