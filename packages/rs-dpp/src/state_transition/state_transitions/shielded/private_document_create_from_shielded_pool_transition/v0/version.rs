use crate::state_transition::private_document_create_from_shielded_pool_transition::v0::PrivateDocumentCreateFromShieldedPoolTransitionV0;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for PrivateDocumentCreateFromShieldedPoolTransitionV0 {
    fn feature_version(&self) -> FeatureVersion {
        0
    }
}
