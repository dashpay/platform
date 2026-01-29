use crate::error::Error;
use dpp::consensus::basic::data_contract::{
    GroupPositionDoesNotExistError, InvalidTokenBaseSupplyError,
    NewTokensDestinationIdentityOptionRequiredError, NonContiguousContractTokenPositionsError,
};
use dpp::dashcore::Network;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use dpp::data_contract::associated_token::token_perpetual_distribution::methods::v0::TokenPerpetualDistributionV0Accessors;
use dpp::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use dpp::data_contract::errors::DataContractError;
use dpp::data_contract::TokenContractPosition;
use dpp::platform_value::Value;
use dpp::prelude::DataContract;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;

const CREATION_RESTRICTION_MODE: &str = "creationRestrictionMode";
const CREATION_RESTRICTION_GROUP: &str = "creationRestrictionGroup";

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_update) trait DataContractUpdateStateTransitionBasicStructureValidationV0
{
    fn validate_basic_structure_v0(
        &self,
        network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DataContractUpdateStateTransitionBasicStructureValidationV0 for DataContractUpdateTransition {
    fn validate_basic_structure_v0(
        &self,
        network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let groups = self.data_contract().groups();
        if !groups.is_empty() {
            let validation_result = DataContract::validate_groups(groups, platform_version)?;

            if !validation_result.is_valid() {
                return Ok(validation_result);
            }
        }

        for schema in self.data_contract().document_schemas().values() {
            let schema_map = match schema.to_map() {
                Ok(map) => map,
                Err(err) => {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        DataContractError::InvalidContractStructure(format!(
                            "document schema must be an object: {err}"
                        ))
                        .into(),
                    ));
                }
            };

            let creation_restriction_mode: u8 = match Value::inner_optional_integer_value::<u8>(
                schema_map,
                CREATION_RESTRICTION_MODE,
            ) {
                Ok(value) => value.unwrap_or(0),
                Err(err) => {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        DataContractError::from(err).into(),
                    ));
                }
            };

            if creation_restriction_mode == 3 {
                let group_position = match Value::inner_optional_integer_value::<u16>(
                    schema_map,
                    CREATION_RESTRICTION_GROUP,
                ) {
                    Ok(value) => value,
                    Err(err) => {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            DataContractError::from(err).into(),
                        ));
                    }
                };

                let Some(group_position) = group_position else {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        DataContractError::InvalidContractStructure(
                            "creationRestrictionGroup is required when creationRestrictionMode is 3"
                                .to_string(),
                        )
                        .into(),
                    ));
                };

                if !self.data_contract().groups().contains_key(&group_position) {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        GroupPositionDoesNotExistError::new(group_position).into(),
                    ));
                }
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
                return Ok(validation_result);
            }

            let validation_result = token_configuration.validate_token_config_groups_exist(
                self.data_contract().groups(),
                platform_version,
            )?;
            if !validation_result.is_valid() {
                return Ok(validation_result);
            }

            if let Some(perpetual_distribution) = token_configuration
                .distribution_rules()
                .perpetual_distribution()
            {
                // we validate the interval (that it's more than one hour or over 100 blocks)
                // also that if it is time based we are using minute intervals
                let validation_result = perpetual_distribution
                    .distribution_type()
                    .validate_structure_interval(network_type, platform_version)?;

                if !validation_result.is_valid() {
                    return Ok(validation_result);
                }

                // We use 0 as the start moment to show that we are starting now with no offset
                let validation_result = perpetual_distribution
                    .distribution_type()
                    .function()
                    .validate(0, platform_version)?;

                if !validation_result.is_valid() {
                    return Ok(validation_result);
                }
            }

            if token_configuration
                .distribution_rules()
                .new_tokens_destination_identity()
                .is_none()
                && !token_configuration
                    .distribution_rules()
                    .minting_allow_choosing_destination()
                && !(token_configuration
                    .distribution_rules()
                    .minting_allow_choosing_destination_rules()
                    .authorized_to_make_change_action_takers()
                    == &AuthorizedActionTakers::NoOne
                    && token_configuration
                        .distribution_rules()
                        .minting_allow_choosing_destination_rules()
                        .admin_action_takers()
                        == &AuthorizedActionTakers::NoOne)
            {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    NewTokensDestinationIdentityOptionRequiredError::new(
                        self.data_contract().id(),
                        *token_contract_position,
                    )
                    .into(),
                ));
            }
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
