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
        if !self.is_contract_update_allowed() {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractIsReadonlyError::new(contract_id).into(),
            );
        }

        if new_config != self {
            return SimpleConsensusValidationResult::new_with_error(
                DataContractConfigUpdateError::new(
                    contract_id,
                    "contract config can not be changed",
                )
                .into(),
            );
        }

        SimpleConsensusValidationResult::new()
    }
}
