use crate::state_transition::identity_topup_from_addresses_transition::IdentityTopUpFromAddressesTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for IdentityTopUpFromAddressesTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            IdentityTopUpFromAddressesTransition::V0(v0) => v0.feature_version(),
        }
    }
}
