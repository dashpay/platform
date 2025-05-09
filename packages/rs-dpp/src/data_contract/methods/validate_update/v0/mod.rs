use std::collections::HashSet;

use crate::block::block_info::BlockInfo;
use crate::consensus::state::state_error::StateError;
use crate::consensus::state::token::PreProgrammedDistributionTimestampInPastError;
use crate::data_contract::accessors::v0::DataContractV0Getters;

use crate::consensus::basic::data_contract::{
    DuplicateKeywordsError, IncompatibleDataContractSchemaError, InvalidDataContractVersionError,
    InvalidDescriptionLengthError, InvalidKeywordCharacterError, InvalidKeywordLengthError,
    TooManyKeywordsError,
};
use crate::consensus::state::data_contract::data_contract_update_action_not_allowed_error::DataContractUpdateActionNotAllowedError;
use crate::consensus::state::data_contract::data_contract_update_permission_error::DataContractUpdatePermissionError;
use crate::consensus::state::data_contract::document_type_update_error::DocumentTypeUpdateError;
use crate::data_contract::accessors::v1::DataContractV1Getters;
use crate::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use crate::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use crate::data_contract::associated_token::token_pre_programmed_distribution::accessors::v0::TokenPreProgrammedDistributionV0Methods;
use crate::data_contract::document_type::schema::validate_schema_compatibility;
use crate::data_contract::schema::DataContractSchemaMethodsV0;
use crate::data_contract::DataContract;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_value::Value;
use platform_version::version::PlatformVersion;
use serde_json::json;

pub trait DataContractUpdateValidationMethodsV0 {
    fn validate_update(
        &self,
        data_contract: &DataContract,
        block_info: &BlockInfo,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl DataContract {
    #[inline(always)]
    pub(super) fn validate_update_v0(
        &self,
        new_data_contract: &DataContract,
        block_info: &BlockInfo,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        // Check if the contract is owned by the same identity
        if self.owner_id() != new_data_contract.owner_id() {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                DataContractUpdatePermissionError::new(self.id(), new_data_contract.owner_id())
                    .into(),
            ));
        }

        // Check version is bumped
        // Failure (version != previous version + 1): Keep ST and transform it to a nonce bump action.
        // How: A user pushed an update that was not the next version.

        let new_version = new_data_contract.version();
        let old_version = self.version();
        if new_version < old_version || new_version - old_version != 1 {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDataContractVersionError::new(old_version + 1, new_version).into(),
            ));
        }

        // Validate that the config was not updated
        // * Includes verifications that:
        //     - Old contract is not read_only
        //     - New contract is not read_only
        //     - Keeps history did not change
        //     - Can be deleted did not change
        //     - Documents keep history did not change
        //     - Documents can be deleted contract default did not change
        //     - Documents mutable contract default did not change
        //     - Requires identity encryption bounded key did not change
        //     - Requires identity decryption bounded key did not change
        // * Failure (contract does not exist): Keep ST and transform it to a nonce bump action.
        // * How: A user pushed an update to a contract that changed its configuration.

        let config_validation_result = self.config().validate_update(
            new_data_contract.config(),
            self.id(),
            platform_version,
        )?;

        if !config_validation_result.is_valid() {
            return Ok(SimpleConsensusValidationResult::new_with_errors(
                config_validation_result.errors,
            ));
        }

        // Validate updates for existing document types to make sure that previously created
        // documents will be still valid with a new version of the data contract
        for (document_type_name, old_document_type) in self.document_types() {
            // Make sure that existing document aren't removed
            let Some(new_document_type) =
                new_data_contract.document_type_optional_for_name(document_type_name)
            else {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    DocumentTypeUpdateError::new(
                        self.id(),
                        document_type_name,
                        "document type can't be removed",
                    )
                    .into(),
                ));
            };

            // Validate document type update rules
            let validate_update_result = old_document_type
                .as_ref()
                .validate_update(new_document_type, platform_version)?;

            if !validate_update_result.is_valid() {
                return Ok(SimpleConsensusValidationResult::new_with_errors(
                    validate_update_result.errors,
                ));
            }
        }

        // Schema $defs should be compatible
        if let Some(old_defs_map) = self.schema_defs() {
            // If new contract doesn't have $defs, it means that it's $defs was removed and compatibility is broken
            let Some(new_defs_map) = new_data_contract.schema_defs() else {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    IncompatibleDataContractSchemaError::new(
                        self.id(),
                        "remove".to_string(),
                        "/$defs".to_string(),
                    )
                    .into(),
                ));
            };

            // If $defs is updated we need to make sure that our data contract is still compatible
            // with previously created data
            if old_defs_map != new_defs_map {
                // both new and old $defs already validated as a part of new and old contract
                let old_defs_json = Value::from(old_defs_map)
                    .try_into_validating_json()
                    .map_err(ProtocolError::ValueError)?;

                let new_defs_json = Value::from(new_defs_map)
                    .try_into_validating_json()
                    .map_err(ProtocolError::ValueError)?;

                let old_defs_schema = json!({
                    "$defs": old_defs_json
                });

                let new_defs_schema = json!({
                    "$defs": new_defs_json
                });

                // We do not allow to remove or modify $ref in document type schemas
                // it means that compatible changes in $defs won't break the overall compatibility
                // Make sure that updated $defs schema is compatible
                let compatibility_validation_result = validate_schema_compatibility(
                    &old_defs_schema,
                    &new_defs_schema,
                    platform_version,
                )?;

                if !compatibility_validation_result.is_valid() {
                    let errors = compatibility_validation_result
                        .errors
                        .into_iter()
                        .map(|operation| {
                            IncompatibleDataContractSchemaError::new(
                                self.id(),
                                operation.name,
                                operation.path,
                            )
                            .into()
                        })
                        .collect();

                    return Ok(SimpleConsensusValidationResult::new_with_errors(errors));
                }
            }
        }

        if self.groups() != new_data_contract.groups() {
            // No groups can have been removed
            for old_group_position in self.groups().keys() {
                if !new_data_contract.groups().contains_key(old_group_position) {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        DataContractUpdateActionNotAllowedError::new(
                            self.id(),
                            "remove group".to_string(),
                        )
                        .into(),
                    ));
                }
            }

            // Ensure no group has been changed
            for (old_group_position, old_group) in self.groups() {
                if let Some(new_group) = new_data_contract.groups().get(old_group_position) {
                    if old_group != new_group {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            DataContractUpdateActionNotAllowedError::new(
                                self.id(),
                                format!(
                                    "change group at position {} is not allowed",
                                    old_group_position
                                ),
                            )
                            .into(),
                        ));
                    }
                }
            }
        }

        if self.tokens() != new_data_contract.tokens() {
            for (token_position, old_token_config) in self.tokens() {
                // Check if a token has been removed
                if !new_data_contract.tokens().contains_key(token_position) {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        DataContractUpdateActionNotAllowedError::new(
                            self.id(),
                            format!("remove token at position {}", token_position),
                        )
                        .into(),
                    ));
                }

                // Check if a token configuration has been changed
                if let Some(new_token_config) = new_data_contract.tokens().get(token_position) {
                    if old_token_config != new_token_config {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            DataContractUpdateActionNotAllowedError::new(
                                self.id(),
                                format!("update token at position {}", token_position),
                            )
                            .into(),
                        ));
                    }
                }
            }

            // Validate any newly added tokens
            for (token_contract_position, token_configuration) in new_data_contract.tokens() {
                if !self.tokens().contains_key(token_contract_position) {
                    if let Some(distribution) = token_configuration
                        .distribution_rules()
                        .pre_programmed_distribution()
                    {
                        if let Some((timestamp, _)) = distribution.distributions().iter().next() {
                            if timestamp < &block_info.time_ms {
                                return Ok(SimpleConsensusValidationResult::new_with_error(
                                    StateError::PreProgrammedDistributionTimestampInPastError(
                                        PreProgrammedDistributionTimestampInPastError::new(
                                            new_data_contract.id(),
                                            *token_contract_position,
                                            *timestamp,
                                            block_info.time_ms,
                                        ),
                                    )
                                    .into(),
                                ));
                            }
                        }
                    }
                }
            }
        }

        if self.keywords() != new_data_contract.keywords() {
            // Validate there are no more than 50 keywords
            if new_data_contract.keywords().len() > 50 {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    TooManyKeywordsError::new(self.id(), new_data_contract.keywords().len() as u8)
                        .into(),
                ));
            }

            // Validate the keywords are all unique and between 3 and 50 characters
            let mut seen_keywords = HashSet::new();
            for keyword in new_data_contract.keywords() {
                // First check keyword length
                if keyword.len() < 3 || keyword.len() > 50 {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidKeywordLengthError::new(self.id(), keyword.to_string()).into(),
                    ));
                }

                if !keyword
                    .chars()
                    .all(|c| !c.is_control() && !c.is_whitespace())
                {
                    // This would mean we have an invalid character
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidKeywordCharacterError::new(
                            new_data_contract.id(),
                            keyword.to_string(),
                        )
                        .into(),
                    ));
                }

                // Then check uniqueness
                if !seen_keywords.insert(keyword) {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        DuplicateKeywordsError::new(self.id(), keyword.to_string()).into(),
                    ));
                }
            }
        }

        if self.description() != new_data_contract.description() {
            // Validate the description is between 3 and 100 characters
            if let Some(description) = new_data_contract.description() {
                let char_count = description.chars().count();
                if !(3..=100).contains(&char_count) {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        InvalidDescriptionLengthError::new(self.id(), description.to_string())
                            .into(),
                    ));
                }
            }
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::basic::basic_error::BasicError;
    use crate::consensus::state::state_error::StateError;
    use crate::consensus::ConsensusError;
    use crate::data_contract::config::v0::DataContractConfigSettersV0;
    use crate::prelude::IdentityNonce;
    use crate::tests::fixtures::get_data_contract_fixture;
    use assert_matches::assert_matches;
    use platform_value::platform_value;
    use platform_value::Identifier;

    mod validate_update {
        use std::collections::BTreeMap;

        use super::*;
        use crate::data_contract::accessors::v0::DataContractV0Setters;
        use crate::data_contract::accessors::v1::DataContractV1Setters;
        use crate::data_contract::associated_token::token_configuration::accessors::v0::{
            TokenConfigurationV0Getters, TokenConfigurationV0Setters,
        };
        use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
        use crate::data_contract::associated_token::token_configuration_convention::v0::TokenConfigurationConventionV0;
        use crate::data_contract::associated_token::token_configuration_convention::TokenConfigurationConvention;
        use crate::data_contract::associated_token::token_configuration_localization::v0::TokenConfigurationLocalizationV0;
        use crate::data_contract::associated_token::token_configuration_localization::TokenConfigurationLocalization;
        use crate::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Setters;
        use crate::data_contract::associated_token::token_pre_programmed_distribution::v0::TokenPreProgrammedDistributionV0;
        use crate::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
        use crate::data_contract::document_type::DocumentTypeMutRef;
        use crate::data_contract::group::accessors::v0::{GroupV0Getters, GroupV0Setters};
        use crate::data_contract::group::v0::GroupV0;
        use crate::data_contract::group::Group;
        use crate::data_contract::TokenConfiguration;
        use crate::identity::accessors::IdentityGettersV0;
        use crate::prelude::Identity;

        #[test]
        fn should_return_invalid_result_if_owner_id_is_not_the_same() {
            let platform_version = PlatformVersion::latest();

            let old_data_contract =
                get_data_contract_fixture(None, IdentityNonce::default(), 1).data_contract_owned();

            let mut new_data_contract = old_data_contract.clone();

            new_data_contract.as_v0_mut().unwrap().owner_id = Identifier::random();

            let result = old_data_contract
                .validate_update(&new_data_contract, &BlockInfo::default(), platform_version)
                .expect("failed validate update");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DataContractUpdatePermissionError(e)
                )] if *e.data_contract_id() == old_data_contract.id() && *e.identity_id() == new_data_contract.owner_id()
            );
        }

        #[test]
        fn should_return_invalid_result_if_contract_version_is_not_greater_for_one() {
            let platform_version = PlatformVersion::latest();

            let old_data_contract =
                get_data_contract_fixture(None, IdentityNonce::default(), 1).data_contract_owned();

            let new_data_contract = old_data_contract.clone();

            let result = old_data_contract
                .validate_update_v0(&new_data_contract, &BlockInfo::default(), platform_version)
                .expect("failed validate update");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::InvalidDataContractVersionError(e)
                )] if e.expected_version() == old_data_contract.version() + 1 && e.version() == new_data_contract.version()
            );
        }

        #[test]
        fn should_return_invalid_result_if_config_was_updated() {
            let platform_version = PlatformVersion::latest();

            let old_data_contract =
                get_data_contract_fixture(None, IdentityNonce::default(), 1).data_contract_owned();

            let mut new_data_contract = old_data_contract.clone();

            new_data_contract.set_version(old_data_contract.version() + 1);
            new_data_contract.config_mut().set_readonly(true);

            let result = old_data_contract
                .validate_update_v0(&new_data_contract, &BlockInfo::default(), platform_version)
                .expect("failed validate update");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DataContractConfigUpdateError(e)
                )] if e.additional_message() == "contract can not be changed to readonly"
            );
        }

        #[test]
        fn should_return_invalid_result_when_document_type_is_removed() {
            let platform_version = PlatformVersion::latest();

            let old_data_contract =
                get_data_contract_fixture(None, IdentityNonce::default(), 1).data_contract_owned();

            let mut new_data_contract = old_data_contract.clone();

            new_data_contract.set_version(old_data_contract.version() + 1);
            new_data_contract
                .document_types_mut()
                .remove("niceDocument");

            let result = old_data_contract
                .validate_update_v0(&new_data_contract, &BlockInfo::default(), platform_version)
                .expect("failed validate update");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DocumentTypeUpdateError(e)
                )] if e.additional_message() == "document type can't be removed"
            );
        }

        #[test]
        fn should_return_invalid_result_when_document_type_has_incompatible_change() {
            let platform_version = PlatformVersion::latest();

            let old_data_contract =
                get_data_contract_fixture(None, IdentityNonce::default(), 1).data_contract_owned();

            let mut new_data_contract = old_data_contract.clone();

            new_data_contract.set_version(old_data_contract.version() + 1);

            let DocumentTypeMutRef::V0(new_document_type) = new_data_contract
                .document_types_mut()
                .get_mut("niceDocument")
                .unwrap()
                .as_mut_ref()
            else {
                panic!("expected v0")
            };
            new_document_type.documents_mutable = false;

            let result = old_data_contract
                .validate_update_v0(&new_data_contract, &BlockInfo::default(), platform_version)
                .expect("failed validate update");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DocumentTypeUpdateError(e)
                )] if e.additional_message() == "document type can not change whether its documents are mutable: changing from true to false"
            );
        }

        #[test]
        fn should_return_invalid_result_when_defs_is_removed() {
            let platform_version = PlatformVersion::latest();

            let mut old_data_contract =
                get_data_contract_fixture(None, IdentityNonce::default(), 1).data_contract_owned();

            // Remove document that uses $defs, so we can safely remove it for testing
            old_data_contract
                .document_types_mut()
                .remove("prettyDocument");

            let mut new_data_contract = old_data_contract.clone();

            new_data_contract.set_version(old_data_contract.version() + 1);
            new_data_contract
                .set_schema_defs(None, false, &mut Vec::new(), platform_version)
                .expect("failed to set schema defs");

            let result = old_data_contract
                .validate_update_v0(&new_data_contract, &BlockInfo::default(), platform_version)
                .expect("failed validate update");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::IncompatibleDataContractSchemaError(e)
                )] if e.operation() == "remove" && e.field_path() == "/$defs"
            );
        }

        #[test]
        fn should_return_invalid_result_when_updated_defs_is_incompatible() {
            let platform_version = PlatformVersion::latest();

            let old_data_contract =
                get_data_contract_fixture(None, IdentityNonce::default(), 1).data_contract_owned();

            let mut new_data_contract = old_data_contract.clone();

            let incompatible_defs_value = platform_value!({
                "lastName": {
                    "type": "number",
                },
            });
            let incompatible_defs = incompatible_defs_value
                .into_btree_string_map()
                .expect("should convert to map");

            new_data_contract.set_version(old_data_contract.version() + 1);
            new_data_contract
                .set_schema_defs(
                    Some(incompatible_defs),
                    false,
                    &mut Vec::new(),
                    platform_version,
                )
                .expect("failed to set schema defs");

            let result = old_data_contract
                .validate_update_v0(&new_data_contract, &BlockInfo::default(), platform_version)
                .expect("failed validate update");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::IncompatibleDataContractSchemaError(e)
                )] if e.operation() == "replace" && e.field_path() == "/$defs/lastName/type"
            );
        }

        #[test]
        fn should_pass_when_all_changes_are_compatible() {
            let platform_version = PlatformVersion::latest();

            let old_data_contract =
                get_data_contract_fixture(None, IdentityNonce::default(), 1).data_contract_owned();

            let mut new_data_contract = old_data_contract.clone();

            new_data_contract.set_version(old_data_contract.version() + 1);

            let result = old_data_contract
                .validate_update_v0(&new_data_contract, &BlockInfo::default(), platform_version)
                .expect("failed validate update");

            assert!(result.is_valid());
        }

        //
        // ──────────────────────────────────────────────────────────────────────────
        //  Group‑related rules
        // ──────────────────────────────────────────────────────────────────────────
        //

        #[test]
        fn should_return_invalid_result_when_group_is_removed() {
            let platform_version = PlatformVersion::latest();

            let identity_1 = Identity::random_identity(3, Some(14), platform_version)
                .expect("expected a platform identity");
            let identity_1_id = identity_1.id();
            let identity_2 = Identity::random_identity(3, Some(506), platform_version)
                .expect("expected a platform identity");
            let identity_2_id = identity_2.id();

            let mut old_data_contract =
                get_data_contract_fixture(None, IdentityNonce::default(), 9).data_contract_owned();
            old_data_contract.set_groups(BTreeMap::from([(
                0,
                Group::V0(GroupV0 {
                    members: [(identity_1_id, 1), (identity_2_id, 1)].into(),
                    required_power: 2,
                }),
            )]));

            // Clone & bump version
            let mut new_data_contract = old_data_contract.clone();
            new_data_contract.set_version(old_data_contract.version() + 1);

            // Remove the first (and normally only) group
            let first_group_pos = *old_data_contract
                .groups()
                .keys()
                .next()
                .expect("fixture must have at least one group");
            new_data_contract
                .groups_mut()
                .unwrap()
                .remove(&first_group_pos);

            let result = old_data_contract
                .validate_update_v0(&new_data_contract, &BlockInfo::default(), platform_version)
                .expect("failed validate update");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DataContractUpdateActionNotAllowedError(e)
                )] if e.action() == "remove group"
            );
        }

        #[test]
        fn should_return_invalid_result_when_group_is_changed() {
            let platform_version = PlatformVersion::latest();

            let identity_1 = Identity::random_identity(3, Some(14), platform_version)
                .expect("expected a platform identity");
            let identity_1_id = identity_1.id();
            let identity_2 = Identity::random_identity(3, Some(506), platform_version)
                .expect("expected a platform identity");
            let identity_2_id = identity_2.id();

            let mut old_data_contract =
                get_data_contract_fixture(None, IdentityNonce::default(), 9).data_contract_owned();
            old_data_contract.set_groups(BTreeMap::from([(
                0,
                Group::V0(GroupV0 {
                    members: [(identity_1_id, 1), (identity_2_id, 1)].into(),
                    required_power: 2,
                }),
            )]));

            // Clone & bump version
            let mut new_data_contract = old_data_contract.clone();
            new_data_contract.set_version(old_data_contract.version() + 1);

            // Mutate the first group in some trivial way so that
            // `old_group != new_group` evaluates to true.
            let first_group_pos = *new_data_contract
                .groups()
                .keys()
                .next()
                .expect("fixture must have at least one group");
            let mut altered_group = new_data_contract
                .groups()
                .get(&first_group_pos)
                .cloned()
                .expect("group must exist");
            // Tweak required power
            altered_group.set_required_power(altered_group.required_power() + 1);
            new_data_contract
                .groups_mut()
                .unwrap()
                .insert(first_group_pos, altered_group);

            let result = old_data_contract
                .validate_update_v0(&new_data_contract, &BlockInfo::default(), platform_version)
                .expect("failed validate update");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DataContractUpdateActionNotAllowedError(e)
                )] if e.action() == format!(
                        "change group at position {} is not allowed",
                        first_group_pos
                    )
            );
        }

        //
        // ──────────────────────────────────────────────────────────────────────────
        //  Token‑related rules
        // ──────────────────────────────────────────────────────────────────────────
        //

        #[test]
        fn should_return_invalid_result_when_token_is_removed() {
            let platform_version = PlatformVersion::latest();

            let mut old_data_contract =
                get_data_contract_fixture(None, IdentityNonce::default(), 9).data_contract_owned();
            old_data_contract.set_tokens(BTreeMap::from([(
                0,
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
            )]));

            let mut new_data_contract = old_data_contract.clone();
            new_data_contract.set_version(old_data_contract.version() + 1);

            // Remove an existing token
            let first_token_pos = *old_data_contract
                .tokens()
                .keys()
                .next()
                .expect("fixture must have at least one token");
            new_data_contract
                .tokens_mut()
                .unwrap()
                .remove(&first_token_pos);

            let result = old_data_contract
                .validate_update_v0(&new_data_contract, &BlockInfo::default(), platform_version)
                .expect("failed validate update");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DataContractUpdateActionNotAllowedError(e)
                )] if e.action() == format!("remove token at position {}", first_token_pos)
            );
        }

        #[test]
        fn should_return_invalid_result_when_token_is_updated() {
            let platform_version = PlatformVersion::latest();

            let mut old_data_contract =
                get_data_contract_fixture(None, IdentityNonce::default(), 9).data_contract_owned();
            old_data_contract.set_tokens(BTreeMap::from([(
                0,
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive()),
            )]));

            let mut new_data_contract = old_data_contract.clone();
            new_data_contract.set_version(old_data_contract.version() + 1);

            // Modify an existing token configuration
            let first_token_pos = *new_data_contract
                .tokens()
                .keys()
                .next()
                .expect("fixture must have at least one token");
            let mut altered_token_cfg = new_data_contract
                .tokens()
                .get(&first_token_pos)
                .cloned()
                .expect("token must exist");
            // Tweak base supply
            altered_token_cfg.set_base_supply(altered_token_cfg.base_supply() + 1);
            new_data_contract
                .tokens_mut()
                .unwrap()
                .insert(first_token_pos, altered_token_cfg);

            let result = old_data_contract
                .validate_update_v0(&new_data_contract, &BlockInfo::default(), platform_version)
                .expect("failed validate update");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::DataContractUpdateActionNotAllowedError(e)
                )] if e.action() == format!("update token at position {}", first_token_pos)
            );
        }

        #[test]
        fn should_return_invalid_result_when_token_is_added_with_past_timestamp() {
            let platform_version = PlatformVersion::latest();

            let mut old_data_contract =
                get_data_contract_fixture(None, IdentityNonce::default(), 9).data_contract_owned();
            let mut token_cfg =
                TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());
            token_cfg.set_conventions(TokenConfigurationConvention::V0(
                TokenConfigurationConventionV0 {
                    localizations: BTreeMap::from([(
                        "en".to_string(),
                        TokenConfigurationLocalization::V0(TokenConfigurationLocalizationV0 {
                            should_capitalize: false,
                            singular_form: "test".to_string(),
                            plural_form: "tests".to_string(),
                        }),
                    )]),
                    decimals: 8,
                },
            ));
            old_data_contract.set_tokens(BTreeMap::from([(0, token_cfg)]));

            let mut new_data_contract = old_data_contract.clone();
            new_data_contract.set_version(old_data_contract.version() + 1);

            // Create a new token with a past timestamp
            let existing_cfg = new_data_contract
                .tokens()
                .values()
                .next()
                .expect("fixture must have at least one token")
                .clone();
            let new_position = old_data_contract
                .tokens()
                .keys()
                .max()
                .expect("fixture must have at least one token")
                + 1;
            let mut new_token_cfg = existing_cfg.clone();
            new_token_cfg
                .distribution_rules_mut()
                .set_pre_programmed_distribution(Some(TokenPreProgrammedDistribution::V0(
                    TokenPreProgrammedDistributionV0 {
                        distributions: BTreeMap::from([(
                            0,
                            BTreeMap::from([(new_data_contract.owner_id(), 100)]),
                        )]),
                    },
                )));
            new_data_contract
                .tokens_mut()
                .unwrap()
                .insert(new_position, new_token_cfg);

            let result = old_data_contract
                .validate_update_v0(
                    &new_data_contract,
                    &BlockInfo::default_with_time(100000),
                    platform_version,
                )
                .expect("failed validate update");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::StateError(
                    StateError::PreProgrammedDistributionTimestampInPastError(e)
                )] if e.token_position() == new_position
            );
        }

        #[test]
        fn should_pass_when_a_well_formed_new_token_is_added() {
            let platform_version = PlatformVersion::latest();

            let old_data_contract =
                get_data_contract_fixture(None, IdentityNonce::default(), 9).data_contract_owned();

            let mut new_data_contract = old_data_contract.clone();
            new_data_contract.set_version(old_data_contract.version() + 1);

            // build a fully valid token configuration
            let valid_token_cfg = {
                let mut cfg =
                    TokenConfiguration::V0(TokenConfigurationV0::default_most_restrictive());

                cfg.set_base_supply(1_000_000); // within limits

                cfg.set_conventions(TokenConfigurationConvention::V0(
                    TokenConfigurationConventionV0 {
                        localizations: BTreeMap::from([(
                            "en".to_string(),
                            TokenConfigurationLocalization::V0(TokenConfigurationLocalizationV0 {
                                should_capitalize: true,
                                singular_form: "credit".to_string(),
                                plural_form: "credits".to_string(),
                            }),
                        )]),
                        decimals: 8,
                    },
                ));

                cfg
            };

            // insert at contiguous position 0 (old contract had no tokens)
            new_data_contract
                .tokens_mut()
                .unwrap()
                .insert(0, valid_token_cfg);

            let result = old_data_contract
                .validate_update_v0(&new_data_contract, &BlockInfo::default(), platform_version)
                .expect("failed validate update");

            assert!(result.is_valid(), "well‑formed token should be accepted");
        }

        //
        // ──────────────────────────────────────────────────────────────────────────
        //  Happy‑path check: no token / group changes
        // ──────────────────────────────────────────────────────────────────────────
        //

        #[test]
        fn should_pass_when_groups_and_tokens_unchanged() {
            let platform_version = PlatformVersion::latest();

            let old_data_contract =
                get_data_contract_fixture(None, IdentityNonce::default(), 9).data_contract_owned();

            let mut new_data_contract = old_data_contract.clone();
            new_data_contract.set_version(old_data_contract.version() + 1);

            let result = old_data_contract
                .validate_update_v0(&new_data_contract, &BlockInfo::default(), platform_version)
                .expect("failed validate update");

            assert!(result.is_valid());
        }
    }
}
