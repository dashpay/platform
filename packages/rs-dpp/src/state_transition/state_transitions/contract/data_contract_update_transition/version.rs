use crate::state_transition::state_transitions::contract::data_contract_update_transition::DataContractUpdateTransition;
use crate::state_transition::FeatureVersioned;
use platform_version::version::protocol_version::FeatureVersion;

impl FeatureVersioned for DataContractUpdateTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            DataContractUpdateTransition::V0(v0) => v0.feature_version(),
        }
    }
}
