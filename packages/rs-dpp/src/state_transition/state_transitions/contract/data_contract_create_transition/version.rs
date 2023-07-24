use crate::state_transition::data_contract_create_transition::DataContractCreateTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for DataContractCreateTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self { DataContractCreateTransition::V0(v0) => v0.feature_version() }
    }
}
