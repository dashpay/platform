use versioned_feature_core::FeatureVersion;

pub mod v1;
pub mod v2;

#[derive(Clone, Debug, Default)]
pub struct DPPStateTransitionConversionVersions {
    pub identity_to_identity_create_transition: FeatureVersion,
    pub identity_to_identity_top_up_transition: FeatureVersion,
    pub identity_to_identity_transfer_transition: FeatureVersion,
    pub identity_to_identity_withdrawal_transition: FeatureVersion,
    pub identity_to_identity_create_transition_with_signer: FeatureVersion,
    pub inputs_to_identity_create_from_addresses_transition_with_signer: FeatureVersion,
    pub address_funds_to_address_funds_transfer_transition: FeatureVersion,
    pub identity_to_identity_top_up_from_addresses_transition: FeatureVersion,
    pub address_funding_from_asset_lock_transition: FeatureVersion,
}
