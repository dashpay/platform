use crate::block::block_info::BlockInfo;
use crate::data_contract::DataContract;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

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
        let result = self.validate_update_ownership_and_version(new_data_contract);
        if !result.is_valid() {
            return Ok(result);
        }

        let result = self.validate_update_config(new_data_contract, platform_version)?;
        if !result.is_valid() {
            return Ok(result);
        }

        let result =
            self.validate_update_existing_document_types(new_data_contract, platform_version)?;
        if !result.is_valid() {
            return Ok(result);
        }

        let result = self.validate_update_schema_defs(new_data_contract, platform_version)?;
        if !result.is_valid() {
            return Ok(result);
        }

        let result = self.validate_update_groups(new_data_contract);
        if !result.is_valid() {
            return Ok(result);
        }

        let result = self.validate_update_tokens(new_data_contract, block_info);
        if !result.is_valid() {
            return Ok(result);
        }

        let result = self.validate_update_keywords(new_data_contract);
        if !result.is_valid() {
            return Ok(result);
        }

        Ok(self.validate_update_description(new_data_contract))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consensus::basic::basic_error::BasicError;
    use crate::consensus::state::state_error::StateError;
    use crate::consensus::ConsensusError;
    use crate::data_contract::accessors::v0::DataContractV0Getters;
    use crate::data_contract::accessors::v1::DataContractV1Getters;
    use crate::data_contract::config::v0::DataContractConfigSettersV0;
    use crate::data_contract::methods::validate_update::DataContractUpdateValidationMethodsV0;
    use crate::data_contract::schema::DataContractSchemaMethodsV0;
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

            let old_data_contract = get_data_contract_fixture(
                None,
                IdentityNonce::default(),
                platform_version.protocol_version,
            )
            .data_contract_owned();

            let mut new_data_contract = old_data_contract.clone();

            new_data_contract.set_owner_id(Identifier::random());

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

            let old_data_contract = get_data_contract_fixture(
                None,
                IdentityNonce::default(),
                platform_version.protocol_version,
            )
            .data_contract_owned();

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

            let old_data_contract = get_data_contract_fixture(
                None,
                IdentityNonce::default(),
                platform_version.protocol_version,
            )
            .data_contract_owned();

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

            let old_data_contract = get_data_contract_fixture(
                None,
                IdentityNonce::default(),
                platform_version.protocol_version,
            )
            .data_contract_owned();

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

            let old_data_contract = get_data_contract_fixture(
                None,
                IdentityNonce::default(),
                platform_version.protocol_version,
            )
            .data_contract_owned();

            let mut new_data_contract = old_data_contract.clone();

            new_data_contract.set_version(old_data_contract.version() + 1);

            match new_data_contract
                .document_types_mut()
                .get_mut("niceDocument")
                .unwrap()
                .as_mut_ref()
            {
                DocumentTypeMutRef::V0(dt) => dt.documents_mutable = false,
                DocumentTypeMutRef::V1(dt) => dt.documents_mutable = false,
                DocumentTypeMutRef::V2(dt) => dt.documents_mutable = false,
            }

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

            let mut old_data_contract = get_data_contract_fixture(
                None,
                IdentityNonce::default(),
                platform_version.protocol_version,
            )
            .data_contract_owned();

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

            let old_data_contract = get_data_contract_fixture(
                None,
                IdentityNonce::default(),
                platform_version.protocol_version,
            )
            .data_contract_owned();

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

            let old_data_contract = get_data_contract_fixture(
                None,
                IdentityNonce::default(),
                platform_version.protocol_version,
            )
            .data_contract_owned();

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
