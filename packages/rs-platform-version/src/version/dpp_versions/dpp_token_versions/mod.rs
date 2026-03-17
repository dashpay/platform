pub mod v1;
pub mod v2;

use versioned_feature_core::FeatureVersion;

#[derive(Clone, Debug, Default)]
pub struct DPPTokenVersions {
    pub identity_token_info_default_structure_version: FeatureVersion,
    pub identity_token_status_default_structure_version: FeatureVersion,
    pub token_contract_info_default_structure_version: FeatureVersion,
    /// Version for the token config update action_id calculation.
    /// v0: uses only the u8 discriminant of the config change item (vulnerable to value swap)
    /// v1: includes the full serialized config change item in the hash
    pub token_config_update_action_id_version: FeatureVersion,
    /// Version for the set-price-for-direct-purchase action_id calculation.
    /// v0: uses only minimum_purchase_amount_and_price().1 (vulnerable to schedule swap)
    /// v1: includes the full serialized TokenPricingSchedule in the hash
    pub token_set_price_action_id_version: FeatureVersion,
}
