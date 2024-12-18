use versioned_feature_core::FeatureVersion;

pub mod v1;

#[derive(Clone, Debug, Default)]
pub struct DriveTokenMethodVersions {
    pub fetch: DriveTokenFetchMethodVersions,
    pub prove: DriveTokenProveMethodVersions,
    pub update: DriveTokenUpdateMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct DriveTokenFetchMethodVersions {
    pub balance: FeatureVersion,
}

#[derive(Clone, Debug, Default)]
pub struct DriveTokenProveMethodVersions {}

#[derive(Clone, Debug, Default)]
pub struct DriveTokenUpdateMethodVersions {
    pub create_token_root_tree: FeatureVersion,
    pub burn: FeatureVersion,
    pub mint: FeatureVersion,
    pub transfer: FeatureVersion,
    pub add_to_token_total_supply: FeatureVersion,
    pub remove_from_token_total_supply: FeatureVersion,
    pub remove_from_identity_token_balance: FeatureVersion,
    pub add_to_identity_token_balance: FeatureVersion,
}
