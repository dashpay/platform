use crate::state_transition::private_document_create_from_shielded_pool_transition::PrivateDocumentCreateFromShieldedPoolTransition;
use crate::state_transition::FeatureVersioned;
use crate::version::FeatureVersion;

impl FeatureVersioned for PrivateDocumentCreateFromShieldedPoolTransition {
    fn feature_version(&self) -> FeatureVersion {
        match self {
            PrivateDocumentCreateFromShieldedPoolTransition::V0(v0) => v0.feature_version(),
        }
    }
}
