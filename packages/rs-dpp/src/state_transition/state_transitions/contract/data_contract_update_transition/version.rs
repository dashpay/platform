use crate::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for DataContractUpdateTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            DataContractUpdateTransition::V0(v0) => v0.feature_version(),
        }
    }
}
