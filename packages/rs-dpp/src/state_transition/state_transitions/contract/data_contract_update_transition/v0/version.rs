use crate::state_transition::state_transitions::contract::data_contract_update_transition::DataContractUpdateTransitionV0;
use crate::state_transition::FeatureVersioned;
use platform_version::version::protocol_version::FeatureVersion;

impl FeatureVersioned for DataContractUpdateTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
