use crate::error::Error;
use dpp::consensus::basic::data_contract::{
    DataContractUpdateEntryKind, DataContractUpdateOverlappingEntriesError,
    InvalidTokenBaseSupplyError, NewTokensDestinationIdentityOptionRequiredError,
};
use dpp::consensus::state::data_contract::data_contract_config_update_error::DataContractConfigUpdateError;
use dpp::dashcore::Network;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use dpp::data_contract::associated_token::token_perpetual_distribution::methods::v0::TokenPerpetualDistributionV0Accessors;
use dpp::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::{FeatureVersion, PlatformVersion};

use super::v1::DataContractUpdateStateTransitionBasicStructureValidationV1;

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_update) trait DataContractUpdateStateTransitionBasicStructureValidationV2
{
    fn validate_basic_structure_v2(
        &self,
        network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DataContractUpdateStateTransitionBasicStructureValidationV2 for DataContractUpdateTransition {
    /// Generation 2 adds delta-based (V1) updates. A full-contract (V0)
    /// update keeps its generation-1 checks.
    ///
    /// A delta can only be checked against itself here: its sections must
    /// not contradict each other, and each new token must be well-formed
    /// on its own. Everything relative to the stored contract (group and
    /// token positions continuing the stored ones, each new group's own
    /// rules, groups a token references, keyword and description limits on
    /// the merged result) waits for state validation, where the delta is
    /// merged and the merged contract is held to the same update rules as
    /// a full-contract update.
    fn validate_basic_structure_v2(
        &self,
        network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let DataContractUpdateTransition::V1(delta) = self else {
            return self.validate_basic_structure_v1(network_type, platform_version);
        };
        let contract_id = delta.data_contract_id;

        if let Some(name) = delta
            .updated_document_schemas
            .keys()
            .find(|name| delta.new_document_schemas.contains_key(*name))
        {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                DataContractUpdateOverlappingEntriesError::new(
                    contract_id,
                    DataContractUpdateEntryKind::DocumentType,
                    name.clone(),
                )
                .into(),
            ));
        }

        if let Some(name) = delta
            .updated_schema_defs
            .keys()
            .find(|name| delta.new_schema_defs.contains_key(*name))
        {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                DataContractUpdateOverlappingEntriesError::new(
                    contract_id,
                    DataContractUpdateEntryKind::SchemaDef,
                    name.clone(),
                )
                .into(),
            ));
        }

        if let Some(keyword) = delta
            .add_keywords
            .iter()
            .find(|keyword| delta.remove_keywords.contains(*keyword))
        {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                DataContractUpdateOverlappingEntriesError::new(
                    contract_id,
                    DataContractUpdateEntryKind::Keyword,
                    keyword.clone(),
                )
                .into(),
            ));
        }

        // A supplied config must meet the floor generation 1 enforces on
        // embedded contracts: pre-V1 configs lack sized integer types.
        if let Some(config) = &delta.config {
            let config_min_version = platform_version.dpp.contract_versions.config.min_version;
            if (config.version() as FeatureVersion) < config_min_version {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    DataContractConfigUpdateError::new(
                        contract_id,
                        format!(
                            "config version {} is not supported, minimum version is {}",
                            config.version(),
                            config_min_version
                        ),
                    )
                    .into(),
                ));
            }
        }

        for (token_contract_position, token_configuration) in &delta.new_tokens {
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
                // the interval must be at least an hour or a hundred blocks,
                // and time based intervals must be whole minutes
                let validation_result = perpetual_distribution
                    .distribution_type()
                    .validate_structure_interval(network_type, platform_version)?;
                if !validation_result.is_valid() {
                    return Ok(validation_result);
                }

                // start moment 0 means the distribution starts now with no offset
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::validation::state_transition::processor::basic_structure::StateTransitionBasicStructureValidationV0;
    use assert_matches::assert_matches;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::ConsensusError;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::accessors::v1::DataContractV1Setters;
    use dpp::data_contract::schema::DataContractSchemaMethodsV0;
    use dpp::platform_value::platform_value;
    use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransitionV1;
    use dpp::tests::fixtures::get_data_contract_fixture;

    /// A delta that adds a document type and a keyword to the fixture contract.
    fn delta() -> DataContractUpdateTransitionV1 {
        let platform_version = PlatformVersion::latest();
        let old_contract = get_data_contract_fixture(None, 0, platform_version.protocol_version)
            .data_contract_owned();
        let mut new_contract = old_contract.clone();
        new_contract.increment_version();
        new_contract
            .set_document_schema(
                "newType",
                platform_value!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "position": 0
                        }
                    },
                    "additionalProperties": false
                }),
                true,
                &mut vec![],
                platform_version,
            )
            .expect("expected to add a document type");
        new_contract.set_keywords(vec!["alpha".to_string()]);

        let DataContractUpdateTransition::V1(delta) =
            DataContractUpdateTransition::from_contract_update(
                &old_contract,
                &new_contract,
                1,
                platform_version,
            )
            .expect("expected a delta-based update transition")
        else {
            panic!("the latest platform version defaults to the delta form");
        };
        delta
    }

    fn validate(delta: DataContractUpdateTransitionV1) -> SimpleConsensusValidationResult {
        DataContractUpdateTransition::V1(delta)
            .validate_basic_structure(Network::Testnet, PlatformVersion::latest())
            .expect("expected basic structure validation to run")
    }

    #[test]
    fn a_delta_whose_sections_do_not_overlap_passes() {
        let result = validate(delta());

        assert!(result.is_valid(), "unexpected errors: {:?}", result.errors);
    }

    #[test]
    fn a_document_type_that_is_both_new_and_updated_is_rejected() {
        let mut delta = delta();
        let contract_id = delta.data_contract_id;
        let (name, schema) = delta
            .new_document_schemas
            .iter()
            .next()
            .map(|(name, schema)| (name.clone(), schema.clone()))
            .expect("the delta adds a document type");
        delta.updated_document_schemas.insert(name.clone(), schema);

        let result = validate(delta);

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(BasicError::DataContractUpdateOverlappingEntriesError(error))]
                if *error == DataContractUpdateOverlappingEntriesError::new(
                    contract_id,
                    DataContractUpdateEntryKind::DocumentType,
                    name.clone(),
                )
        );
    }

    #[test]
    fn a_schema_definition_that_is_both_new_and_updated_is_rejected() {
        let mut delta = delta();
        let contract_id = delta.data_contract_id;
        let definition = platform_value!({ "type": "string" });
        delta
            .new_schema_defs
            .insert("shared".to_string(), definition.clone());
        delta
            .updated_schema_defs
            .insert("shared".to_string(), definition);

        let result = validate(delta);

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(BasicError::DataContractUpdateOverlappingEntriesError(error))]
                if *error == DataContractUpdateOverlappingEntriesError::new(
                    contract_id,
                    DataContractUpdateEntryKind::SchemaDef,
                    "shared".to_string(),
                )
        );
    }

    #[test]
    fn a_keyword_that_is_both_added_and_removed_is_rejected() {
        let mut delta = delta();
        let contract_id = delta.data_contract_id;
        delta.remove_keywords.push("alpha".to_string());

        let result = validate(delta);

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(BasicError::DataContractUpdateOverlappingEntriesError(error))]
                if *error == DataContractUpdateOverlappingEntriesError::new(
                    contract_id,
                    DataContractUpdateEntryKind::Keyword,
                    "alpha".to_string(),
                )
        );
    }
}
