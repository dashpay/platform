use crate::consensus::state::data_contract::data_contract_config_update_error::DataContractConfigUpdateError;
use crate::consensus::state::data_contract::data_contract_is_readonly_error::DataContractIsReadonlyError;
use crate::data_contract::config::v0::DataContractConfigGettersV0;
use crate::data_contract::config::DataContractConfig;
use crate::validation::SimpleConsensusValidationResult;
use platform_value::Identifier;

impl DataContractConfig {
    #[inline(always)]
    pub(super) fn validate_update_v0(
        &self,
        new_config: &DataContractConfig,
        contract_id: Identifier,
    ) -> SimpleConsensusValidationResult {
        // Validate: Old contract is not read_only

        if self.readonly() {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractIsReadonlyError::new(contract_id).into(),
            );
        }

        // Validate: New contract is not read_only

        if new_config.readonly() {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractConfigUpdateError::new(
                    contract_id,
                    "contract can not be changed to readonly",
                )
                .into(),
            );
        }

        // Validate: Keeps history did not change

        if new_config.keeps_history() != self.keeps_history() {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractConfigUpdateError::new(
                    contract_id,
                    format!(
                        "contract can not change whether it keeps history: changing from {} to {}",
                        self.keeps_history(),
                        new_config.keeps_history()
                    ),
                )
                .into(),
            );
        }

        // Validate: Can be deleted did not change

        if new_config.can_be_deleted() != self.can_be_deleted() {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractConfigUpdateError::new(
                    contract_id,
                    format!(
                        "contract can not change whether it can be delete: changing from {} to {}",
                        self.can_be_deleted(),
                        new_config.can_be_deleted()
                    ),
                )
                .into(),
            );
        }

        // Validate: Documents keep history did not change

        if new_config.documents_keep_history_contract_default()
            != self.documents_keep_history_contract_default()
        {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractConfigUpdateError::new(
                    contract_id,
                    "contract can not change the default of whether documents keeps history",
                )
                .into(),
            );
        }

        // Validate: Documents mutable contract default did not change

        if new_config.documents_mutable_contract_default()
            != self.documents_mutable_contract_default()
        {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractConfigUpdateError::new(
                    contract_id,
                    "contract can not change the default of whether documents are mutable",
                )
                .into(),
            );
        }

        // Validate: Documents can be deleted contract default did not change

        if new_config.documents_can_be_deleted_contract_default()
            != self.documents_can_be_deleted_contract_default()
        {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractConfigUpdateError::new(
                    contract_id,
                    "contract can not change the default of whether documents can be deleted",
                )
                .into(),
            );
        }

        // Validate: Requires identity encryption bounded key did not change

        if new_config.requires_identity_encryption_bounded_key()
            != self.requires_identity_encryption_bounded_key()
        {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractConfigUpdateError::new(
                    contract_id,
                    "contract can not change the requirement of needing a document encryption bounded key",
                )
                    .into(),
            );
        }

        // Validate: Requires identity decryption bounded key did not change

        if new_config.requires_identity_decryption_bounded_key()
            != self.requires_identity_decryption_bounded_key()
        {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractConfigUpdateError::new(
                    contract_id,
                    "contract can not change the requirement of needing a document decryption bounded key",
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
    use crate::consensus::state::state_error::StateError;
    use crate::consensus::ConsensusError;
    use crate::data_contract::config::v0::DataContractConfigV0;
    use crate::data_contract::storage_requirements::keys_for_document_type::StorageKeyRequirements;
    use assert_matches::assert_matches;
    use platform_value::Identifier;

    mod validate_update {
        use super::*;

        #[test]
        fn should_fail_when_old_contract_is_readonly() {
            let old_config: DataContractConfig = DataContractConfigV0 {
                readonly: true,
                ..Default::default()
            }
            .into();
            let new_config = DataContractConfigV0::default().into();

            let contract_id = Identifier::default();
            let result = old_config.validate_update_v0(&new_config, contract_id);

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(StateError::DataContractIsReadonlyError(e))] if *e.data_contract_id() == contract_id
            );
        }

        #[test]
        fn should_fail_when_new_contract_is_readonly() {
            let old_config: DataContractConfig = DataContractConfigV0::default().into();
            let new_config = DataContractConfigV0 {
                readonly: true,
                ..Default::default()
            }
            .into();

            let contract_id = Identifier::default();
            let result = old_config.validate_update_v0(&new_config, contract_id);

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DataContractConfigUpdateError(e)
                )] if *e.data_contract_id() == contract_id && e.additional_message() == "contract can not be changed to readonly"
            );
        }

        #[test]
        fn should_fail_when_keeps_history_changes() {
            let old_config: DataContractConfig = DataContractConfigV0 {
                keeps_history: true,
                ..Default::default()
            }
            .into();
            let new_config = DataContractConfigV0 {
                keeps_history: false,
                ..Default::default()
            }
            .into();

            let contract_id = Identifier::default();
            let result = old_config.validate_update_v0(&new_config, contract_id);

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DataContractConfigUpdateError(e)
                )] if *e.data_contract_id() == contract_id && e.additional_message() == "contract can not change whether it keeps history: changing from true to false"
            );
        }

        #[test]
        fn should_fail_when_can_be_deleted_changes() {
            let old_config: DataContractConfig = DataContractConfigV0 {
                can_be_deleted: true,
                ..Default::default()
            }
            .into();
            let new_config = DataContractConfigV0 {
                can_be_deleted: false,
                ..Default::default()
            }
            .into();

            let contract_id = Identifier::default();
            let result = old_config.validate_update_v0(&new_config, contract_id);

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DataContractConfigUpdateError(e)
                )] if *e.data_contract_id() == contract_id && e.additional_message() == "contract can not change whether it can be delete: changing from true to false"
            );
        }

        #[test]
        fn should_fail_when_documents_keep_history_contract_default_changes() {
            let old_config: DataContractConfig = DataContractConfigV0 {
                documents_keep_history_contract_default: true,
                ..Default::default()
            }
            .into();
            let new_config = DataContractConfigV0 {
                documents_keep_history_contract_default: false,
                ..Default::default()
            }
            .into();

            let contract_id = Identifier::default();
            let result = old_config.validate_update_v0(&new_config, contract_id);

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DataContractConfigUpdateError(e)
                )] if *e.data_contract_id() == contract_id && e.additional_message() == "contract can not change the default of whether documents keeps history"
            );
        }

        #[test]
        fn should_fail_when_documents_mutable_contract_default_changes() {
            let old_config: DataContractConfig = DataContractConfigV0 {
                documents_mutable_contract_default: true,
                ..Default::default()
            }
            .into();
            let new_config = DataContractConfigV0 {
                documents_mutable_contract_default: false,
                ..Default::default()
            }
            .into();

            let contract_id = Identifier::default();
            let result = old_config.validate_update_v0(&new_config, contract_id);

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DataContractConfigUpdateError(e)
                )] if *e.data_contract_id() == contract_id && e.additional_message() == "contract can not change the default of whether documents are mutable"
            );
        }

        #[test]
        fn should_fail_when_documents_can_be_deleted_contract_default_changes() {
            let old_config: DataContractConfig = DataContractConfigV0 {
                documents_can_be_deleted_contract_default: true,
                ..Default::default()
            }
            .into();
            let new_config = DataContractConfigV0 {
                documents_can_be_deleted_contract_default: false,
                ..Default::default()
            }
            .into();

            let contract_id = Identifier::default();
            let result = old_config.validate_update_v0(&new_config, contract_id);

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DataContractConfigUpdateError(e)
                )] if *e.data_contract_id() == contract_id && e.additional_message() == "contract can not change the default of whether documents can be deleted"
            );
        }

        #[test]
        fn should_fail_when_requires_identity_encryption_bounded_key_changes() {
            let old_config: DataContractConfig = DataContractConfigV0 {
                requires_identity_encryption_bounded_key: Some(StorageKeyRequirements::Multiple),
                ..Default::default()
            }
            .into();
            let new_config = DataContractConfigV0 {
                requires_identity_encryption_bounded_key: None,
                ..Default::default()
            }
            .into();

            let contract_id = Identifier::default();
            let result = old_config.validate_update_v0(&new_config, contract_id);

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DataContractConfigUpdateError(e)
                )] if *e.data_contract_id() == contract_id && e.additional_message() == "contract can not change the requirement of needing a document encryption bounded key"
            );
        }

        #[test]
        fn should_fail_when_requires_identity_decryption_bounded_key_changes() {
            let old_config: DataContractConfig = DataContractConfigV0 {
                requires_identity_decryption_bounded_key: Some(StorageKeyRequirements::Unique),
                ..Default::default()
            }
            .into();
            let new_config = DataContractConfigV0 {
                requires_identity_decryption_bounded_key: Some(StorageKeyRequirements::Multiple),
                ..Default::default()
            }
            .into();

            let contract_id = Identifier::default();
            let result = old_config.validate_update_v0(&new_config, contract_id);

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DataContractConfigUpdateError(e)
                )] if *e.data_contract_id() == contract_id && e.additional_message() == "contract can not change the requirement of needing a document decryption bounded key"
            );
        }

        #[test]
        fn should_pass_when_no_fields_change() {
            let old_config: DataContractConfig = DataContractConfigV0::default().into();
            let new_config = DataContractConfigV0::default().into();
            let contract_id = Identifier::default();

            let result = old_config.validate_update_v0(&new_config, contract_id);

            assert!(result.is_valid());
        }
    }
}
