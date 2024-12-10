use versioned_feature_core::FeatureVersion;

pub mod v1;

#[derive(Clone, Debug, Default)]
pub struct DriveTokenMethodVersions {
    pub fetch: DriveTokenFetchMethodVersions,
    pub prove: DriveTokenProveMethodVersions,
    pub insert: DriveTokenInsertMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveTokenFetchMethodVersions {}

#[derive(Clone, Debug, Default)]
pub struct DriveTokenProveMethodVersions {}

#[derive(Clone, Debug, Default)]
pub struct DriveTokenInsertMethodVersions {
    pub create_token_root_tree: FeatureVersion,
}
