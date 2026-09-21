use crate::error::Error;
use dpp::consensus::basic::contract_group::{
    ContractGroupMemberNotInContractError, ContractGroupMembershipsOverLimitError,
    DuplicateContractGroupMembershipError, InvalidContractGroupAdminsError,
    InvalidContractGroupDescriptionLengthError, InvalidContractGroupNameLengthError,
    RedundantContractGroupMembershipError,
};
use dpp::consensus::basic::contract_moderation::DocumentActionFeesWithoutModerationError;
use dpp::consensus::basic::data_contract::DataContractInvalidRequiredFieldsUpdateError;
use dpp::consensus::ConsensusError;
use dpp::contract_group::ContractGroupMember;
use dpp::dashcore::Network;
use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use dpp::data_contract::config::v2::DataContractConfigGettersV2;
use dpp::data_contract::document_type::action_fees::DocumentActionFees;
use dpp::identifier::Identifier;
use dpp::state_transition::data_contract_create_transition::accessors::{
    DataContractCreateTransitionAccessorsV0, DataContractCreateTransitionAccessorsV1,
};
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::state_transition::StateTransitionOwned;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use std::collections::BTreeSet;

use super::v1::DataContractCreateStateTransitionBasicStructureValidationV1;

const PROPERTIES: &str = "properties";
const REQUIRED_SINCE: &str = "requiredSince";

pub(in crate::execution::validation::state_transition::state_transitions::data_contract_create) trait DataContractCreateStateTransitionBasicStructureValidationV2
{
    fn validate_basic_structure_v2(
        &self,
        network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DataContractCreateStateTransitionBasicStructureValidationV2 for DataContractCreateTransition {
    fn validate_basic_structure_v2(
        &self,
        network_type: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        // First run all v1 (and transitively v0) validations
        let v1_result = self.validate_basic_structure_v1(network_type, platform_version)?;
        if !v1_result.is_valid() {
            return Ok(v1_result);
        }

        // `requiredSince` names the contract version a property is required
        // from. A freshly created contract is version 1, so the only value
        // that names an existing version is 1 (which is equivalent to plain
        // membership in `required`). Later values would pre-schedule
        // requiredness at a future version — coherent for the wire format,
        // but banned: requiredness changes must arrive with the update that
        // creates the version they name.
        //
        // This raw-JSON scan is an early, cheap rejection only — it cannot
        // see an annotation reached through a `$defs` `$ref`. The
        // authoritative enforcement is
        // `validate_required_since_within_contract_version` in dpp, which
        // runs on the *parsed* properties (references resolved) whenever the
        // contract is built from its serialized form, including this
        // transition's transform into action.
        for (document_type_name, schema) in self.data_contract().document_schemas() {
            let Some(properties) = schema
                .get_optional_value(PROPERTIES)
                .ok()
                .flatten()
                .and_then(|properties| properties.as_map())
            else {
                continue;
            };

            for (property_name, property_schema) in properties {
                let Some(required_since) = property_schema
                    .as_map()
                    .and_then(|map| {
                        map.iter()
                            .find(|(key, _)| key.as_text() == Some(REQUIRED_SINCE))
                    })
                    .and_then(|(_, value)| value.as_integer::<u32>())
                else {
                    continue;
                };

                if required_since != 1 {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        DataContractInvalidRequiredFieldsUpdateError::new(
                            document_type_name.clone(),
                            format!(
                                "property '{}' of a newly created contract cannot carry requiredSince {} — a fresh contract is version 1",
                                property_name.as_text().unwrap_or_default(),
                                required_since
                            ),
                        )
                        .into(),
                    ));
                }
            }
        }

        // Contract groups (version 1 transitions; a version 0 transition carries none).
        if let Some(error) = contract_group_basic_structure_error(self, platform_version) {
            return Ok(SimpleConsensusValidationResult::new_with_error(error));
        }

        // Drive stores every pre-programmed release as a sum tree of its recipients' amounts.
        // A release totalling more than `i64::MAX` passed every check and then failed inside
        // Drive as an internal error, which nobody pays for and which only makes the
        // transition disappear from every proposal.
        for (token_contract_position, token_configuration) in self.data_contract().tokens() {
            let Some(distribution) = token_configuration
                .distribution_rules()
                .pre_programmed_distribution()
            else {
                continue;
            };

            let validation_result =
                distribution.validate_amounts(*token_contract_position, platform_version)?;
            if !validation_result.is_valid() {
                return Ok(validation_result);
            }
        }

        // Contract moderation: a config that declares it must be well formed (a list or a
        // document type moderators can delete, a non-empty moderator set within the limit, and
        // an elected declaration within its bounds and naming document types of the contract).
        // That the named moderators exist is checked against the state.
        if let Some(moderation) = self.data_contract().config().moderation() {
            let result =
                moderation.validate(self.data_contract().document_schemas(), platform_version)?;
            if !result.is_valid() {
                return Ok(result);
            }
        }

        // Document action fees: a document type may only charge for the moderators when the
        // contract declares moderation, since the moderation team is who that pot is for.
        if self.data_contract().config().moderation().is_none() {
            if let Some(document_type_name) =
                DocumentActionFees::first_document_type_charging_moderators(
                    self.data_contract().document_schemas(),
                )
            {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    DocumentActionFeesWithoutModerationError::new(document_type_name.clone())
                        .into(),
                ));
            }
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}

/// Checks everything about the transition's contract group registration and memberships that
/// needs no state: the registrant is an owner, owner and text limits hold, every member exists
/// in the created contract, and no membership repeats or is implied by a whole-contract
/// membership of the same group. Whether the groups exist and who owns them is checked against
/// the state.
fn contract_group_basic_structure_error(
    transition: &DataContractCreateTransition,
    platform_version: &PlatformVersion,
) -> Option<ConsensusError> {
    {
        let limits = &platform_version.system_limits;
        let owner_id = transition.owner_id();

        if let Some(registration) = transition.contract_group() {
            // The owner is the signer and never on the wire; only the admins need checking.
            let admins = &registration.admins;
            if admins.len() > limits.max_contract_group_admins as usize
                || admins.contains(&owner_id)
            {
                return Some(
                    InvalidContractGroupAdminsError::new(
                        admins.len() as u32,
                        limits.max_contract_group_admins,
                    )
                    .into(),
                );
            }
            if let Some(name) = &registration.name {
                let length = name.chars().count();
                if length == 0 || length > limits.max_contract_group_name_length as usize {
                    return Some(
                        InvalidContractGroupNameLengthError::new(
                            name.clone(),
                            limits.max_contract_group_name_length,
                        )
                        .into(),
                    );
                }
            }
            if let Some(description) = &registration.description {
                let length = description.chars().count();
                if length == 0 || length > limits.max_contract_group_description_length as usize {
                    return Some(
                        InvalidContractGroupDescriptionLengthError::new(
                            description.clone(),
                            limits.max_contract_group_description_length,
                        )
                        .into(),
                    );
                }
            }
        }

        let memberships = transition.contract_group_memberships();
        if memberships.len() > limits.max_contract_group_memberships_per_contract as usize {
            return Some(
                ContractGroupMembershipsOverLimitError::new(
                    memberships.len() as u32,
                    limits.max_contract_group_memberships_per_contract,
                )
                .into(),
            );
        }

        let contract = transition.data_contract();
        let whole_contract_groups: BTreeSet<Identifier> = memberships
            .iter()
            .filter(|membership| membership.member == ContractGroupMember::Contract)
            .map(|membership| membership.contract_group_id)
            .collect();
        let mut seen = BTreeSet::new();
        for membership in memberships {
            match &membership.member {
                ContractGroupMember::Contract => {}
                ContractGroupMember::DocumentType(document_type_name) => {
                    if !contract.document_schemas().contains_key(document_type_name) {
                        return Some(
                            ContractGroupMemberNotInContractError::new(
                                contract.id(),
                                membership.member.clone(),
                            )
                            .into(),
                        );
                    }
                }
                ContractGroupMember::Token(token_position) => {
                    if !contract.tokens().contains_key(token_position) {
                        return Some(
                            ContractGroupMemberNotInContractError::new(
                                contract.id(),
                                membership.member.clone(),
                            )
                            .into(),
                        );
                    }
                }
            }
            if !seen.insert((membership.contract_group_id, membership.member.clone())) {
                return Some(
                    DuplicateContractGroupMembershipError::new(
                        membership.contract_group_id,
                        membership.member.clone(),
                    )
                    .into(),
                );
            }
            if membership.member != ContractGroupMember::Contract
                && whole_contract_groups.contains(&membership.contract_group_id)
            {
                return Some(
                    RedundantContractGroupMembershipError::new(
                        membership.contract_group_id,
                        membership.member.clone(),
                    )
                    .into(),
                );
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::validation::state_transition::processor::basic_structure::StateTransitionBasicStructureValidationV0;
    use assert_matches::assert_matches;
    use dpp::balances::credits::TokenAmount;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::ConsensusError;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::accessors::v1::DataContractV1Setters;
    use dpp::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use dpp::data_contract::associated_token::token_configuration::TokenConfiguration;
    use dpp::data_contract::associated_token::token_configuration_convention::accessors::v0::TokenConfigurationConventionV0Getters;
    use dpp::data_contract::associated_token::token_configuration_localization::v0::TokenConfigurationLocalizationV0;
    use dpp::data_contract::associated_token::token_configuration_localization::TokenConfigurationLocalization;
    use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Setters;
    use dpp::data_contract::associated_token::token_pre_programmed_distribution::v0::TokenPreProgrammedDistributionV0;
    use dpp::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
    use dpp::data_contract::config::moderation::{ContractModerationConfig, ContractModerators};
    use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
    use dpp::platform_value::{platform_value, Value};
    use dpp::prelude::IdentityNonce;
    use dpp::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use platform_version::version::PlatformVersion;
    use platform_version::TryIntoPlatformVersioned;
    use std::collections::BTreeMap;

    fn create_transition_with_required_since(
        required_since: u32,
    ) -> (DataContractCreateTransition, &'static PlatformVersion) {
        let platform_version = PlatformVersion::latest();
        let identity_nonce = IdentityNonce::default();

        let data_contract =
            get_data_contract_fixture(None, identity_nonce, platform_version.protocol_version)
                .data_contract_owned();

        let mut data_contract_for_serialization: dpp::data_contract::serialized_version::DataContractInSerializationFormat = data_contract
            .try_into_platform_versioned(platform_version)
            .expect("failed to convert data contract");

        data_contract_for_serialization
            .document_schemas_mut()
            .insert(
                "note".to_string(),
                platform_value!({
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "position": 0,
                            "maxLength": 60,
                            "requiredSince": required_since,
                        }
                    },
                    "required": ["message"],
                    "additionalProperties": false
                }),
            );

        let transition: DataContractCreateTransition = DataContractCreateTransitionV0 {
            data_contract: data_contract_for_serialization,
            identity_nonce,
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into();

        (transition, platform_version)
    }

    #[test]
    fn should_accept_required_since_of_one_on_a_new_contract() {
        let (transition, platform_version) = create_transition_with_required_since(1);

        let result = transition
            .validate_basic_structure_v2(Network::Testnet, platform_version)
            .expect("failed to validate basic structure");

        assert!(
            result.is_valid(),
            "requiredSince 1 on a fresh contract is equivalent to plain \
             required and must be accepted, got {:?}",
            result.errors
        );
    }

    #[test]
    fn should_reject_required_since_above_one_on_a_new_contract() {
        let (transition, platform_version) = create_transition_with_required_since(2);

        let result = transition
            .validate_basic_structure_v2(Network::Testnet, platform_version)
            .expect("failed to validate basic structure");

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidRequiredFieldsUpdateError(e)
            )] if e.details().contains("cannot carry requiredSince 2")
        );
    }

    /// A create transition whose contract has one token per entry of `releases`, each
    /// releasing its amounts at time 100, one recipient per amount.
    fn create_transition_with_pre_programmed_releases(
        releases: &[&[TokenAmount]],
        platform_version: &PlatformVersion,
    ) -> DataContractCreateTransition {
        let identity_nonce = IdentityNonce::default();

        let mut data_contract =
            get_data_contract_fixture(None, identity_nonce, platform_version.protocol_version)
                .data_contract_owned();

        for (position, amounts) in releases.iter().enumerate() {
            let mut configuration = TokenConfiguration::V0(
                TokenConfigurationV0::default_most_restrictive().with_base_supply(0),
            );
            configuration.conventions_mut().localizations_mut().insert(
                "en".to_string(),
                TokenConfigurationLocalization::V0(TokenConfigurationLocalizationV0 {
                    should_capitalize: true,
                    singular_form: "test".to_string(),
                    plural_form: "tests".to_string(),
                }),
            );
            let release = amounts
                .iter()
                .enumerate()
                .map(|(recipient, amount)| (Identifier::from([recipient as u8 + 1; 32]), *amount))
                .collect::<BTreeMap<_, _>>();
            configuration
                .distribution_rules_mut()
                .set_pre_programmed_distribution(Some(TokenPreProgrammedDistribution::V0(
                    TokenPreProgrammedDistributionV0 {
                        distributions: BTreeMap::from([(100, release)]),
                    },
                )));
            data_contract.add_token(position as u16, configuration);
        }

        DataContractCreateTransitionV0 {
            data_contract: data_contract
                .try_into_platform_versioned(platform_version)
                .expect("failed to convert data contract"),
            identity_nonce,
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into()
    }

    /// A create transition whose `niceDocument` document type charges `action_fees`, on a
    /// contract that declares moderation or not
    fn create_transition_with_action_fees(
        action_fees: Value,
        moderated: bool,
        platform_version: &PlatformVersion,
    ) -> DataContractCreateTransition {
        let identity_nonce = IdentityNonce::default();
        let mut data_contract =
            get_data_contract_fixture(None, identity_nonce, platform_version.protocol_version)
                .data_contract_owned();
        if moderated {
            data_contract.set_config(data_contract.config().clone().with_moderation(Some(
                ContractModerationConfig {
                    banlist: true,
                    suspensions: false,
                    moderators: ContractModerators::ContractOwner,
                    warnings: false,
                },
            )));
        }
        let mut serialized: DataContractInSerializationFormat = data_contract
            .try_into_platform_versioned(platform_version)
            .expect("failed to convert data contract");
        serialized
            .document_schemas_mut()
            .get_mut("niceDocument")
            .expect("expected the niceDocument schema")
            .insert("actionFees".to_string(), action_fees)
            .expect("expected to declare the action fees");

        DataContractCreateTransitionV0 {
            data_contract: serialized,
            identity_nonce,
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: Default::default(),
        }
        .into()
    }

    // The moderators pot is for the moderation team, and a contract that declares no
    // moderation has none: the credits could never be claimed.
    #[test]
    fn should_reject_a_moderators_action_fee_on_a_contract_without_moderation() {
        let platform_version = PlatformVersion::latest();
        let transition = create_transition_with_action_fees(
            platform_value!({"create": {"owner": 5_u64, "moderators": 10_u64}}),
            false,
            platform_version,
        );

        let result = transition
            .validate_basic_structure(Network::Testnet, platform_version)
            .expect("failed to validate basic structure");

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DocumentActionFeesWithoutModerationError(e)
            )] if e.document_type_name() == "niceDocument"
        );
    }

    #[test]
    fn should_accept_action_fees_the_contract_can_pay_out() {
        let platform_version = PlatformVersion::latest();
        // An owner fee needs no moderation, and a moderators fee is fine once it is declared.
        for (action_fees, moderated) in [
            (platform_value!({"create": {"owner": 5_u64}}), false),
            (
                platform_value!({"create": {"owner": 5_u64, "moderators": 10_u64}}),
                true,
            ),
        ] {
            let transition =
                create_transition_with_action_fees(action_fees, moderated, platform_version);
            let result = transition
                .validate_basic_structure(Network::Testnet, platform_version)
                .expect("failed to validate basic structure");
            assert!(result.is_valid(), "{:?}", result.errors);
        }
    }

    #[test]
    fn should_reject_a_pre_programmed_release_totalling_over_the_limit() {
        let platform_version = PlatformVersion::latest();
        let over_limit_releases: [&[TokenAmount]; 2] = [
            &[i64::MAX as TokenAmount + 1],
            &[i64::MAX as TokenAmount, 1],
        ];

        for over_limit_release in over_limit_releases {
            // The second token is the offending one
            let transition = create_transition_with_pre_programmed_releases(
                &[&[445], over_limit_release],
                platform_version,
            );

            let result = transition
                .validate_basic_structure(Network::Testnet, platform_version)
                .expect("failed to validate basic structure");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::PreProgrammedDistributionAmountOverLimitError(e)
                )] if e.token_position() == 1 && e.timestamp() == 100
            );
        }
    }

    #[test]
    fn should_accept_a_pre_programmed_release_totalling_the_limit() {
        let platform_version = PlatformVersion::latest();

        let transition = create_transition_with_pre_programmed_releases(
            &[&[i64::MAX as TokenAmount - 1, 1]],
            platform_version,
        );

        let result = transition
            .validate_basic_structure(Network::Testnet, platform_version)
            .expect("failed to validate basic structure");

        assert!(result.is_valid(), "unexpected errors: {:?}", result.errors);
    }

    /// Protocol version 13 runs basic structure v1, which is shipped and keeps admitting the
    /// release. The create then fails inside Drive as an internal error, as it always has.
    #[test]
    fn should_still_accept_a_release_total_over_the_limit_on_protocol_version_13() {
        let platform_version = PlatformVersion::get(13).expect("expected protocol version 13");

        let transition =
            create_transition_with_pre_programmed_releases(&[&[u64::MAX]], platform_version);

        let result = transition
            .validate_basic_structure(Network::Testnet, platform_version)
            .expect("failed to validate basic structure");

        assert!(result.is_valid(), "unexpected errors: {:?}", result.errors);
    }
}
