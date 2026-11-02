use crate::state_transition::contract_fee_claim_transition::ContractFeeClaimTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for ContractFeeClaimTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            ContractFeeClaimTransition::V0(v0) => v0.feature_version(),
        }
    }
}
