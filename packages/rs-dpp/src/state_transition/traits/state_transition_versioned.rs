use platform_version::version::FeatureVersion;

pub trait FeatureVersioned {
    fn feature_version(&self) -> FeatureVersion;
}
