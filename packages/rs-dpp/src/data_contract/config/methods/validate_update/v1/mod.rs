use crate::consensus::state::data_contract::data_contract_config_update_error::DataContractConfigUpdateError;
use crate::data_contract::config::v1::DataContractConfigGettersV1;
use crate::data_contract::config::DataContractConfig;
use crate::validation::SimpleConsensusValidationResult;
use platform_value::Identifier;
use platform_version::version::FeatureVersion;
use platform_version::version::PlatformVersion;

impl DataContractConfig {
    #[inline(always)]
    pub(super) fn validate_update_v1(
        &self,
        new_config: &DataContractConfig,
        contract_id: Identifier,
        platform_version: &PlatformVersion,
    ) -> SimpleConsensusValidationResult {
        // Run all v0 checks first
        let v0_result = self.validate_update_v0(new_config, contract_id);
        if !v0_result.is_valid() {
            return v0_result;
        }

        // Validate: new config version meets minimum version requirement.
        // Since protocol version 12, V0 config is no longer accepted because it lacks
        // sized_integer_types support.
        let min_version = platform_version.dpp.contract_versions.config.min_version;
        if (new_config.version() as FeatureVersion) < min_version {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractConfigUpdateError::new(
                    contract_id,
                    format!(
                        "config version {} is not supported, minimum version is {}",
                        new_config.version(),
                        min_version
                    ),
                )
                .into(),
            );
        }

        // Validate: sized_integer_types cannot change from true to false.
        // V1→V0 (true→false) is DANGEROUS: documents serialized with sized types (version byte 1/2)
        // would break when deserialized with I64 types.
        // V0→V1 (false→true) is SAFE: version byte 0 docs use from_bytes_v0 which forces I64
        // regardless of current config.
        if self.sized_integer_types() && !new_config.sized_integer_types() {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractConfigUpdateError::new(
                    contract_id,
                    "contract can not disable sized integer types once enabled, as this would break deserialization of existing documents",
                )
                .into(),
            );
        }

        SimpleConsensusValidationResult::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::config::v0::DataContractConfigV0;
    use crate::data_contract::config::v1::DataContractConfigV1;

    #[test]
    fn test_v1_to_v0_rejected() {
        let platform_version = PlatformVersion::latest();
        let contract_id = Identifier::new([1u8; 32]);
        let config_v1 = DataContractConfig::V1(DataContractConfigV1::default());
        let config_v0 = DataContractConfig::V0(DataContractConfigV0::default());

        // ConfigV1 has sized_integer_types=true, ConfigV0 has sized_integer_types=false
        assert!(config_v1.sized_integer_types());
        assert!(!config_v0.sized_integer_types());

        let result = config_v1.validate_update_v1(&config_v0, contract_id, platform_version);
        assert!(
            !result.is_valid(),
            "V1→V0 config change should be rejected. Errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_v1_sized_true_to_v1_sized_false_rejected() {
        let platform_version = PlatformVersion::latest();
        let contract_id = Identifier::new([1u8; 32]);
        let config_v1_true = DataContractConfig::V1(DataContractConfigV1::default());
        let mut v1_false = DataContractConfigV1::default();
        v1_false.sized_integer_types = false;
        let config_v1_false = DataContractConfig::V1(v1_false);

        assert!(config_v1_true.sized_integer_types());
        assert!(!config_v1_false.sized_integer_types());

        let result =
            config_v1_true.validate_update_v1(&config_v1_false, contract_id, platform_version);
        assert!(
            !result.is_valid(),
            "V1(sized=true)→V1(sized=false) should be rejected. Errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_v0_to_v1_allowed() {
        let platform_version = PlatformVersion::latest();
        let contract_id = Identifier::new([1u8; 32]);
        let config_v0 = DataContractConfig::V0(DataContractConfigV0::default());
        let config_v1 = DataContractConfig::V1(DataContractConfigV1::default());

        // V0→V1 (false→true) is safe because version byte 0 docs use from_bytes_v0
        let result = config_v0.validate_update_v1(&config_v1, contract_id, platform_version);
        assert!(
            result.is_valid(),
            "V0→V1 config change should be allowed (safe direction). Errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_v0_to_v0_rejected_on_latest_version() {
        let platform_version = PlatformVersion::latest();
        let contract_id = Identifier::new([1u8; 32]);
        let config_v0 = DataContractConfig::V0(DataContractConfigV0::default());
        let config_v0_2 = DataContractConfig::V0(DataContractConfigV0::default());

        // On latest platform version (v12+), V0 config is below min_version=1
        let result = config_v0.validate_update_v1(&config_v0_2, contract_id, platform_version);
        assert!(
            !result.is_valid(),
            "V0→V0 should be rejected on latest platform version where min config version is 1. Errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_v1_to_v1_same_allowed() {
        let platform_version = PlatformVersion::latest();
        let contract_id = Identifier::new([1u8; 32]);
        let config_v1 = DataContractConfig::V1(DataContractConfigV1::default());
        let config_v1_2 = DataContractConfig::V1(DataContractConfigV1::default());

        let result = config_v1.validate_update_v1(&config_v1_2, contract_id, platform_version);
        assert!(
            result.is_valid(),
            "V1→V1 (same config) should be allowed. Errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_all_v0_checks_still_work() {
        let platform_version = PlatformVersion::latest();
        let contract_id = Identifier::new([1u8; 32]);
        let config_v1 = DataContractConfig::V1(DataContractConfigV1::default());

        // Changing keeps_history should be rejected
        let mut modified = DataContractConfigV1::default();
        modified.keeps_history = !modified.keeps_history;
        let config_modified = DataContractConfig::V1(modified);

        let result = config_v1.validate_update_v1(&config_modified, contract_id, platform_version);
        assert!(
            !result.is_valid(),
            "Changing keeps_history should be rejected by validate_update_v1"
        );

        // Changing readonly (to true) should be rejected
        let mut modified2 = DataContractConfigV1::default();
        modified2.readonly = true;
        let config_readonly = DataContractConfig::V1(modified2);

        let result2 = config_v1.validate_update_v1(&config_readonly, contract_id, platform_version);
        assert!(
            !result2.is_valid(),
            "Changing readonly to true should be rejected by validate_update_v1"
        );

        // Changing can_be_deleted should be rejected
        let mut modified3 = DataContractConfigV1::default();
        modified3.can_be_deleted = !modified3.can_be_deleted;
        let config_can_be_deleted = DataContractConfig::V1(modified3);

        let result3 =
            config_v1.validate_update_v1(&config_can_be_deleted, contract_id, platform_version);
        assert!(
            !result3.is_valid(),
            "Changing can_be_deleted should be rejected by validate_update_v1"
        );

        // Changing documents_mutable_contract_default should be rejected
        let mut modified4 = DataContractConfigV1::default();
        modified4.documents_mutable_contract_default =
            !modified4.documents_mutable_contract_default;
        let config_docs_mutable = DataContractConfig::V1(modified4);

        let result4 =
            config_v1.validate_update_v1(&config_docs_mutable, contract_id, platform_version);
        assert!(
            !result4.is_valid(),
            "Changing documents_mutable_contract_default should be rejected by validate_update_v1"
        );
    }
}
