use crate::error::Error;
use dpp::consensus::basic::data_contract::{
    InvalidTokenBaseSupplyError, NonContiguousContractTokenPositionsError,
};
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::TokenContractPosition;
use dpp::prelude::DataContract;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_update) trait DataContractUpdateStateTransitionBasicStructureValidationV0
{
    fn validate_basic_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DataContractUpdateStateTransitionBasicStructureValidationV0 for DataContractUpdateTransition {
    fn validate_basic_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let groups = self.data_contract().groups();
        if !groups.is_empty() {
            let validation_result = DataContract::validate_groups(groups, platform_version)?;

            if !validation_result.is_valid() {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    validation_result
                        .errors
                        .first()
                        .cloned()
                        .expect("error should exist"),
                ));
            }
        }

        for (expected_position, (token_contract_position, token_configuration)) in
            self.data_contract().tokens().iter().enumerate()
        {
            if expected_position as TokenContractPosition != *token_contract_position {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    NonContiguousContractTokenPositionsError::new(
                        expected_position as TokenContractPosition,
                        *token_contract_position,
                    )
                    .into(),
                ));
            }

            if token_configuration.base_supply() > i64::MAX as u64 {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    InvalidTokenBaseSupplyError::new(token_configuration.base_supply()).into(),
                ));
            }

            let validation_result = token_configuration
                .conventions()
                .validate_localizations(platform_version)?;
            if !validation_result.is_valid() {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    validation_result
                        .errors
                        .first()
                        .cloned()
                        .expect("error should exist"),
                ));
            }

            let validation_result = token_configuration.validate_token_config_groups_exist(
                self.data_contract().groups(),
                platform_version,
            )?;
            if !validation_result.is_valid() {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    validation_result
                        .errors
                        .first()
                        .cloned()
                        .expect("error should exist"),
                ));
            }
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
