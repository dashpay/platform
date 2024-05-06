use crate::state_transition::state_transitions::contract::data_contract_create_transition::DataContractCreateTransition;
use crate::state_transition::FeatureVersioned;
use platform_version::version::FeatureVersion;

impl FeatureVersioned for DataContractCreateTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            DataContractCreateTransition::V0(v0) => v0.feature_version(),
        }
    }
}
