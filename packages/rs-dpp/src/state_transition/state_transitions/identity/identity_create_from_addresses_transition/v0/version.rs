use crate::state_transition::identity_create_from_addresses_transition::v0::IdentityCreateFromAddressesTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for IdentityCreateFromAddressesTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
