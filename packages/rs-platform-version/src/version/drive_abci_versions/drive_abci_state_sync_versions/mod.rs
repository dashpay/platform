pub mod v1;

use versioned_feature_core::FeatureVersion;

#[derive(Clone, Debug, Default)]
pub struct DriveAbciStateSyncVersions {
    pub protocol_version: FeatureVersion,
}
