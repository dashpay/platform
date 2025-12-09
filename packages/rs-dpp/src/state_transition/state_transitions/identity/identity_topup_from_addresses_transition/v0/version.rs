use crate::state_transition::identity_topup_from_addresses_transition::v0::IdentityTopUpFromAddressesTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for IdentityTopUpFromAddressesTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
