use crate::identity::Identity;
use crate::version::{FeatureVersion, PlatformVersion, LATEST_PLATFORM_VERSION};

/// Versions
impl Identity {
    /// Returns the default FeatureVersion for a given platform version.
    ///
    /// # Arguments
    ///
    /// * `platform_version` - A reference to the PlatformVersion object
    pub fn default_version_on(platform_version: &PlatformVersion) -> FeatureVersion {
        platform_version.identity.default_current_version
    }

    /// Returns the default FeatureVersion for the latest platform version.
    pub fn default_version() -> FeatureVersion {
        LATEST_PLATFORM_VERSION.identity.default_current_version
    }

    /// Returns the latest FeatureVersion supported by a given platform version.
    ///
    /// # Arguments
    ///
    /// * `platform_version` - A reference to the PlatformVersion object
    pub fn latest_version_on(platform_version: &PlatformVersion) -> FeatureVersion {
        platform_version.identity.max_version
    }

    /// Returns the latest FeatureVersion supported by the latest platform version.
    pub fn latest_version() -> FeatureVersion {
        LATEST_PLATFORM_VERSION.identity.max_version
    }
}
