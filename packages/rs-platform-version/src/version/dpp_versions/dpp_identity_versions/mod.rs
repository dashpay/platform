use versioned_feature_core::FeatureVersion;

pub mod v1;

#[derive(Clone, Debug, Default)]
pub struct DPPIdentityVersions {
    /// This is the structure of the Identity as it is defined for code paths
    pub identity_structure_version: FeatureVersion,
    pub identity_key_structure_version: FeatureVersion,
    pub identity_key_type_method_versions: IdentityKeyTypeMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct IdentityKeyTypeMethodVersions {
    pub random_public_key_data: FeatureVersion,
    pub random_public_and_private_key_data: FeatureVersion,
}