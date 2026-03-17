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
}
