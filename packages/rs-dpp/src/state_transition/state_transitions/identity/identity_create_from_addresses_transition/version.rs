use crate::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for IdentityCreateFromAddressesTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            IdentityCreateFromAddressesTransition::V0(v0) => v0.feature_version(),
        }
    }
}
