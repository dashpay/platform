use crate::state_transition::state_transitions::contract::data_contract_create_transition::DataContractCreateTransitionV0;
use crate::state_transition::FeatureVersioned;
use versioned_feature_core::FeatureVersion;

impl FeatureVersioned for DataContractCreateTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
