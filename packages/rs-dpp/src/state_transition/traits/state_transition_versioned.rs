use versioned_feature_core::FeatureVersion;

pub trait FeatureVersioned {
    fn feature_version(&self) -> FeatureVersion;
}
