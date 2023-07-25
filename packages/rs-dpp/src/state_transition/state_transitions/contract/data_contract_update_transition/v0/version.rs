use crate::state_transition::data_contract_update_transition::DataContractUpdateTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for DataContractUpdateTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
