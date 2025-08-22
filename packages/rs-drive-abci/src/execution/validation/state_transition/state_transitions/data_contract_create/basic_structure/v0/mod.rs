use crate::error::Error;
use dpp::consensus::basic::data_contract::{
    DuplicateKeywordsError, InvalidDataContractVersionError, InvalidDescriptionLengthError,
    InvalidKeywordCharacterError, InvalidKeywordLengthError, InvalidTokenBaseSupplyError,
    NewTokensDestinationIdentityOptionRequiredError, NonContiguousContractTokenPositionsError,
    TooManyKeywordsError,
};
use dpp::consensus::basic::BasicError;
use dpp::consensus::ConsensusError;
use dpp::dashcore::Network;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use dpp::data_contract::associated_token::token_perpetual_distribution::methods::v0::TokenPerpetualDistributionV0Accessors;
use dpp::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use dpp::data_contract::{TokenContractPosition, INITIAL_DATA_CONTRACT_VERSION};
use dpp::prelude::DataContract;
use dpp::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use std::collections::HashSet;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_create) trait DataContractCreateStateTransitionBasicStructureValidationV0
{
    fn validate_basic_structure_v0(
        &self,
        network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DataContractCreateStateTransitionBasicStructureValidationV0 for DataContractCreateTransition {
    fn validate_basic_structure_v0(
        &self,
        network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        if self.data_contract().version() != INITIAL_DATA_CONTRACT_VERSION {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDataContractVersionError::new(
                    INITIAL_DATA_CONTRACT_VERSION,
                    self.data_contract().version(),
                )
                .into(),
            ));
        }

        let groups = self.data_contract().groups();
        if !groups.is_empty() {
            let validation_result = DataContract::validate_groups(groups, platform_version)?;

            if !validation_result.is_valid() {
                return Ok(validation_result);
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

        // Validate there are no more than 50 keywords
        if self.data_contract().keywords().len() > 50 {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::BasicError(BasicError::TooManyKeywordsError(
                    TooManyKeywordsError::new(
                        self.data_contract().id(),
                        self.data_contract().keywords().len() as u8,
                    ),
                )),
            ));
        }

        // Validate the keywords are all unique and between 3 and 50 characters
        let mut seen_keywords = HashSet::new();
        for keyword in self.data_contract().keywords() {
            // First check keyword length
            if keyword.len() < 3 || keyword.len() > 50 {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::BasicError(BasicError::InvalidKeywordLengthError(
                        InvalidKeywordLengthError::new(
                            self.data_contract().id(),
                            keyword.to_string(),
                        ),
                    )),
                ));
            }

            if !keyword
                .chars()
                .all(|c| !c.is_control() && !c.is_whitespace())
            {
                // This would mean we have an invalid character
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::BasicError(BasicError::InvalidKeywordCharacterError(
                        InvalidKeywordCharacterError::new(
                            self.data_contract().id(),
                            keyword.to_string(),
                        ),
                    )),
                ));
            }

            // Then check uniqueness
            if !seen_keywords.insert(keyword) {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::BasicError(BasicError::DuplicateKeywordsError(
                        DuplicateKeywordsError::new(self.data_contract().id(), keyword.to_string()),
                    )),
                ));
            }
        }

        // Validate the description is between 3 and 100 characters
        if let Some(description) = self.data_contract().description() {
            if !(description.len() >= 3 && description.len() <= 100) {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::BasicError(BasicError::InvalidDescriptionLengthError(
                        InvalidDescriptionLengthError::new(
                            self.data_contract().id(),
                            description.to_string(),
                        ),
                    )),
                ));
            }
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;

    mod validate_basic_structure {
        use super::*;
        use dpp::consensus::basic::BasicError;
        use dpp::consensus::ConsensusError;
        use dpp::data_contract::accessors::v0::DataContractV0Setters;
        use dpp::data_contract::INITIAL_DATA_CONTRACT_VERSION;
        use dpp::prelude::IdentityNonce;
        use dpp::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
        use dpp::tests::fixtures::get_data_contract_fixture;
        use platform_version::version::PlatformVersion;
        use platform_version::TryIntoPlatformVersioned;

        #[test]
        fn should_return_invalid_result_if_contract_version_is_not_initial() {
            let platform_version = PlatformVersion::latest();
            let identity_nonce = IdentityNonce::default();

            let mut data_contract =
                get_data_contract_fixture(None, identity_nonce, platform_version.protocol_version)
                    .data_contract_owned();

            data_contract.set_version(6);

            let data_contract_for_serialization = data_contract
                .try_into_platform_versioned(platform_version)
                .expect("failed to convert data contract");

            let transition: DataContractCreateTransition = DataContractCreateTransitionV0 {
                data_contract: data_contract_for_serialization,
                identity_nonce,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: Default::default(),
            }
            .into();

            let result = transition
                .validate_basic_structure_v0(Network::Testnet, &platform_version)
                .expect("failed to validate advanced structure");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(BasicError::InvalidDataContractVersionError(e))] if e.expected_version() == INITIAL_DATA_CONTRACT_VERSION && e.version() == 6
            );
        }
    }
}
