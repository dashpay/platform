use crate::consensus::state::data_contract::data_contract_config_update_error::DataContractConfigUpdateError;
use crate::consensus::state::data_contract::data_contract_is_readonly_error::DataContractIsReadonlyError;
use crate::data_contract::config::v0::DataContractConfigGettersV0;
use crate::data_contract::config::DataContractConfig;
use crate::validation::SimpleConsensusValidationResult;
use platform_value::Identifier;

impl DataContractConfig {
    pub(super) fn validate_config_update_v0(
        &self,
        new_config: &DataContractConfig,
        contract_id: Identifier,
    ) -> SimpleConsensusValidationResult {
        if self.readonly() {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractIsReadonlyError::new(contract_id).into(),
            );
        }

        if new_config.readonly() {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractConfigUpdateError::new(
                    contract_id,
                    "contract can not be changed to readonly",
                )
                .into(),
            );
        }

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

        SimpleConsensusValidationResult::new()
    }
}
