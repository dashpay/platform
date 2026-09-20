use crate::block::block_info::BlockInfo;
use crate::consensus::basic::data_contract::DataContractInvalidRequiredFieldsUpdateError;
use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::accessors::v1::DataContractV1Getters;
use crate::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use crate::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::DataContract;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

impl DataContract {
    /// Generation 1 (protocol version 14, `requiredSince`): generation 0
    /// plus validation of `requiredSince` annotations on document types
    /// introduced by the update, which have no old counterpart for the
    /// per-type pass to see. (Required-set changes on *existing* document
    /// types are judged inside the shared per-type dispatcher, which
    /// resolves its own generation from the platform version, so this
    /// method needs no logic of its own for them.)
    ///
    /// It also bounds the pre-programmed distribution amounts of the tokens
    /// the update adds, which from the same protocol version get their
    /// distribution storage written by the update (Drive `update_contract`
    /// v2) and so have to be storable.
    ///
    /// Delegating to generation 0 is safe because that generation is
    /// shipped and therefore frozen. The checks are independent and
    /// short-circuiting, so appending the extra ones changes only which
    /// error is reported when an update violates several rules at once —
    /// never whether it is rejected.
    #[inline(always)]
    pub(super) fn validate_update_v1(
        &self,
        new_data_contract: &DataContract,
        block_info: &BlockInfo,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let result = self.validate_update_v0(new_data_contract, block_info, platform_version)?;
        if !result.is_valid() {
            return Ok(result);
        }

        let result = self.validate_update_new_document_types_required_since(new_data_contract);
        if !result.is_valid() {
            return Ok(result);
        }

        self.validate_update_new_tokens_pre_programmed_amounts(new_data_contract, platform_version)
    }

    /// A token introduced by this update has its pre-programmed releases
    /// written to Drive as sum trees, so each release must total at most
    /// `i64::MAX`.
    ///
    /// Only the added tokens are judged. Before protocol version 14 an
    /// update wrote no distribution storage and therefore admitted a token
    /// with a release whose amounts each fit but total more; its contract
    /// carries that token in every later update and has to remain updatable.
    fn validate_update_new_tokens_pre_programmed_amounts(
        &self,
        new_data_contract: &DataContract,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        for (token_contract_position, token_configuration) in new_data_contract.tokens() {
            if self.tokens().contains_key(token_contract_position) {
                continue;
            }
            let Some(distribution) = token_configuration
                .distribution_rules()
                .pre_programmed_distribution()
            else {
                continue;
            };

            let result =
                distribution.validate_amounts(*token_contract_position, platform_version)?;
            if !result.is_valid() {
                return Ok(result);
            }
        }

        Ok(SimpleConsensusValidationResult::new())
    }

    /// Document types introduced by this update have no old counterpart,
    /// so the per-type update validation never sees them. Their
    /// `requiredSince` annotations must name the version this update
    /// creates — anything else would pre-schedule (or backdate) a
    /// wire-layout change without validation.
    fn validate_update_new_document_types_required_since(
        &self,
        new_data_contract: &DataContract,
    ) -> SimpleConsensusValidationResult {
        for (document_type_name, new_document_type) in new_data_contract.document_types() {
            if self
                .document_type_optional_for_name(document_type_name)
                .is_some()
            {
                continue;
            }
            for (property_name, property) in new_document_type.as_ref().properties() {
                if let Some(required_since) = property.required_since {
                    if required_since != new_data_contract.version() {
                        return SimpleConsensusValidationResult::new_with_error(
                            DataContractInvalidRequiredFieldsUpdateError::new(
                                document_type_name.clone(),
                                format!(
                                    "new document type property '{property_name}' must carry requiredSince {}, the contract version this update creates",
                                    new_data_contract.version()
                                ),
                            )
                            .into(),
                        );
                    }
                }
            }
        }

        SimpleConsensusValidationResult::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::balances::credits::TokenAmount;
    use crate::consensus::basic::basic_error::BasicError;
    use crate::consensus::ConsensusError;
    use crate::data_contract::accessors::v0::DataContractV0Setters;
    use crate::data_contract::accessors::v1::DataContractV1Setters;
    use crate::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
    use crate::data_contract::associated_token::token_configuration::v0::TokenConfigurationV0;
    use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
    use crate::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Setters;
    use crate::data_contract::associated_token::token_pre_programmed_distribution::v0::TokenPreProgrammedDistributionV0;
    use crate::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
    use crate::data_contract::methods::validate_update::DataContractUpdateValidationMethodsV0;
    use crate::data_contract::schema::DataContractSchemaMethodsV0;
    use crate::prelude::IdentityNonce;
    use crate::tests::fixtures::get_data_contract_fixture;
    use assert_matches::assert_matches;
    use platform_value::{platform_value, Identifier};
    use std::collections::BTreeMap;

    #[test]
    fn should_validate_required_since_on_document_types_added_by_the_update() {
        let platform_version = PlatformVersion::latest();

        let old_data_contract = get_data_contract_fixture(
            None,
            IdentityNonce::default(),
            platform_version.protocol_version,
        )
        .data_contract_owned();

        let new_type_schema = |required_since: u32| {
            platform_value!({
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "position": 0,
                        "maxLength": 60_u32,
                        "requiredSince": required_since,
                    }
                },
                "required": ["message"],
                "additionalProperties": false
            })
        };

        // A new document type pre-scheduling requiredness at version 99
        // has no old counterpart, so the per-type update validation
        // never runs on it — this pass must catch it
        let mut new_data_contract = old_data_contract.clone();
        new_data_contract.set_version(old_data_contract.version() + 1);
        new_data_contract
            .set_document_schema(
                "note",
                new_type_schema(99),
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("should add document type");

        let result = old_data_contract
            .validate_update(&new_data_contract, &BlockInfo::default(), platform_version)
            .expect("failed validate update");

        assert_matches!(
            result.errors.as_slice(),
            [ConsensusError::BasicError(
                BasicError::DataContractInvalidRequiredFieldsUpdateError(e)
            )] if e.details().contains("must carry requiredSince 2")
        );

        // The same new document type annotated with the version this
        // update creates is accepted
        let mut new_data_contract = old_data_contract.clone();
        new_data_contract.set_version(old_data_contract.version() + 1);
        new_data_contract
            .set_document_schema(
                "note",
                new_type_schema(old_data_contract.version() + 1),
                false,
                &mut Vec::new(),
                platform_version,
            )
            .expect("should add document type");

        let result = old_data_contract
            .validate_update(&new_data_contract, &BlockInfo::default(), platform_version)
            .expect("failed validate update");

        assert!(
            result.is_valid(),
            "a new document type annotated with the version this update \
             creates must be accepted, got {:?}",
            result.errors
        );
    }

    /// A token releasing `amounts` at time 100, one recipient per amount.
    fn token_releasing(amounts: &[TokenAmount]) -> TokenConfiguration {
        let mut configuration = TokenConfiguration::V0(
            TokenConfigurationV0::default_most_restrictive().with_base_supply(0),
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
        configuration
    }

    /// The fixture contract holding `old_tokens`, and the update of it that holds `new_tokens`.
    fn contract_and_its_update(
        old_tokens: BTreeMap<u16, TokenConfiguration>,
        new_tokens: BTreeMap<u16, TokenConfiguration>,
        protocol_version: u32,
    ) -> (DataContract, DataContract) {
        let mut old_data_contract =
            get_data_contract_fixture(None, IdentityNonce::default(), protocol_version)
                .data_contract_owned();
        old_data_contract.set_tokens(old_tokens);

        let mut new_data_contract = old_data_contract.clone();
        new_data_contract.set_tokens(new_tokens);
        new_data_contract.set_version(old_data_contract.version() + 1);

        (old_data_contract, new_data_contract)
    }

    #[test]
    fn should_reject_a_token_added_by_the_update_whose_release_totals_over_the_limit() {
        let platform_version = PlatformVersion::latest();
        let over_limit_releases: [&[TokenAmount]; 2] = [
            &[i64::MAX as TokenAmount + 1],
            &[i64::MAX as TokenAmount, 1],
        ];

        for amounts in over_limit_releases {
            let (old_data_contract, new_data_contract) = contract_and_its_update(
                BTreeMap::new(),
                BTreeMap::from([(0, token_releasing(amounts))]),
                platform_version.protocol_version,
            );

            let result = old_data_contract
                .validate_update(&new_data_contract, &BlockInfo::default(), platform_version)
                .expect("failed validate update");

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::PreProgrammedDistributionAmountOverLimitError(e)
                )] if e.token_position() == 0 && e.timestamp() == 100
            );
        }
    }

    #[test]
    fn should_accept_a_token_added_by_the_update_whose_release_totals_the_limit() {
        let platform_version = PlatformVersion::latest();

        let (old_data_contract, new_data_contract) = contract_and_its_update(
            BTreeMap::new(),
            BTreeMap::from([(0, token_releasing(&[i64::MAX as TokenAmount - 1, 1]))]),
            platform_version.protocol_version,
        );

        let result = old_data_contract
            .validate_update(&new_data_contract, &BlockInfo::default(), platform_version)
            .expect("failed validate update");

        assert!(result.is_valid(), "unexpected errors: {:?}", result.errors);
    }

    /// Before protocol version 14 a contract update wrote no distribution storage, so it
    /// admitted a token with a release whose amounts each fit but total over the limit. Such
    /// a contract has to stay updatable: the check covers the tokens an update adds, never
    /// the ones it carries over.
    #[test]
    fn should_not_judge_the_release_totals_of_tokens_the_contract_already_had() {
        let platform_version = PlatformVersion::latest();

        let legacy_token = token_releasing(&[i64::MAX as TokenAmount, 1]);
        let (old_data_contract, new_data_contract) = contract_and_its_update(
            BTreeMap::from([(0, legacy_token.clone())]),
            BTreeMap::from([(0, legacy_token), (1, token_releasing(&[445]))]),
            platform_version.protocol_version,
        );

        let result = old_data_contract
            .validate_update(&new_data_contract, &BlockInfo::default(), platform_version)
            .expect("failed validate update");

        assert!(result.is_valid(), "unexpected errors: {:?}", result.errors);
    }

    #[test]
    fn should_still_accept_a_release_total_over_the_limit_on_protocol_version_13() {
        let platform_version = PlatformVersion::get(13).expect("expected protocol version 13");

        let (old_data_contract, new_data_contract) = contract_and_its_update(
            BTreeMap::new(),
            BTreeMap::from([(0, token_releasing(&[i64::MAX as TokenAmount, 1]))]),
            platform_version.protocol_version,
        );

        let result = old_data_contract
            .validate_update(&new_data_contract, &BlockInfo::default(), platform_version)
            .expect("failed validate update");

        assert!(result.is_valid(), "unexpected errors: {:?}", result.errors);
    }
}
