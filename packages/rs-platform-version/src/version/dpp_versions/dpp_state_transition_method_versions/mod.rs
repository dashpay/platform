use versioned_feature_core::FeatureVersion;

pub mod v1;

#[derive(Clone, Debug, Default)]
pub struct DPPStateTransitionMethodVersions {
    pub public_key_in_creation_methods: PublicKeyInCreationMethodVersions,
}

#[derive(Clone, Debug, Default)]
pub struct PublicKeyInCreationMethodVersions {
    pub from_public_key_signed_with_private_key: FeatureVersion,
    pub from_public_key_signed_external: FeatureVersion,
    pub hash: FeatureVersion,
    pub duplicated_key_ids_witness: FeatureVersion,
    pub duplicated_keys_witness: FeatureVersion,
    pub validate_identity_public_keys_structure: FeatureVersion,
}
