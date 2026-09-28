use crate::state_transition::contract_fee_claim_transition::v0::ContractFeeClaimTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for ContractFeeClaimTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
