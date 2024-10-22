use versioned_feature_core::FeatureVersion;

pub mod v1;
pub mod v2;

#[derive(Clone, Debug, Default)]
pub struct DPPStateTransitionConversionVersions {
    pub identity_to_identity_create_transition: FeatureVersion,
    pub identity_to_identity_top_up_transition: FeatureVersion,
    pub identity_to_identity_withdrawal_transition: FeatureVersion,
    pub identity_to_identity_create_transition_with_signer: FeatureVersion,
}