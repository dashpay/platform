use crate::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for DataContractCreateTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
