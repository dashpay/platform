use crate::state_transition::data_contract_create_transition::DataContractCreateTransitionV1;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for DataContractCreateTransitionV1 {
    fn feature_version(&self) -> FeatureVersion {
        1
    }
}
