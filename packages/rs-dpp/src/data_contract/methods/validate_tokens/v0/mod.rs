use crate::consensus::basic::data_contract::{
    InvalidTokenBaseSupplyError, NewTokensDestinationIdentityOptionRequiredError,
    NonContiguousContractTokenPositionsError,
};
use crate::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use crate::data_contract::associated_token::token_perpetual_distribution::methods::v0::TokenPerpetualDistributionV0Accessors;
use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use crate::data_contract::{DataContract, TokenContractPosition};
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_value::Identifier;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl DataContract {
    #[inline(always)]
    pub(super) fn validate_tokens_v0(
        contract_id: Identifier,
        tokens: &BTreeMap<TokenContractPosition, TokenConfiguration>,
        allow_offset_start: bool,
        network: dashcore::Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        // Get the start position from the first token (allows non-zero start for updates)
        let start_position = if allow_offset_start {
            tokens.keys().next().copied().unwrap_or(0)
        } else {
            0
        };

        for (index, (token_contract_position, token_configuration)) in tokens.iter().enumerate() {
            let expected_position = start_position + index as TokenContractPosition;
            if expected_position != *token_contract_position {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    NonContiguousContractTokenPositionsError::new(
                        expected_position,
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

            if let Some(perpetual_distribution) = token_configuration
                .distribution_rules()
                .perpetual_distribution()
            {
                // we validate the interval (that it's more than one hour or over 100 blocks)
                // also that if it is time based we are using minute intervals
                let validation_result = perpetual_distribution
                    .distribution_type()
                    .validate_structure_interval(network, platform_version)?;

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
                        contract_id,
                        *token_contract_position,
                    )
                    .into(),
                ));
            }
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
